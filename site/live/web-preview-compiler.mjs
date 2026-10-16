// Compute pages concurrently, but merge/paint their results in one commit queue.
// A document version owns its readiness gates; transport delivery is not a commit.
export class PreviewCompiler {
  constructor({pages, initial={}, snapshot, revision, active, paused, compile, validate, update, commit, state, error, cancel,
    concurrency=()=>1, canRun=()=>true}) {
    Object.assign(this, {snapshot, revision, active, paused, compile, validate, update, commit, state, error, cancel, concurrency, canRun});
    this.records = pages.map(value => ({...initial[value.sourceId || value.id], key:value.sourceId || value.id, value, valid:true}));
    this.previous = snapshot();
    this.version = this.planVersion = 0;
    this.jobs = new Map();
    this.waiters = new Set();
    this.listeners = new Set();
    this.commits = Promise.resolve();
    this.totals = {attempts:0,computeMs:0,cancelledComputeMs:0};
  }
  get running() { return this.jobs.size > 0; }
  get current() { return this.records.find(r => r.key === this.active()) || this.records[0]; }
  get currentReady() { return !this.pending && !!this.current?.valid; }
  get complete() { return !this.pending && this.records.length > 0 && this.records.every(r => r.valid); }
  get failure() { return this.planError || this.records.find(r => r.error)?.error; }
  subscribe(listener) { this.listeners.add(listener); return () => this.listeners.delete(listener); }
  event(type, detail={}) { this.trace?.events.push({type, at:performance.now(), ...detail}); }
  notify() {
    if (this.complete && this.trace && this.trace.completedAt == null) this.trace.completedAt = performance.now();
    this.state?.(this);
    for (const listener of this.listeners) listener(this);
    for (const resolve of this.waiters) resolve();
    this.waiters.clear();
  }
  schedule(delay=30) {
    this.version++;
    this.cancel?.(this.version);
    this.trace = {version:this.version, startedAt:performance.now(), events:[]};
    this.firstEdit ||= this.trace;
    this.pending = true;
    this.planError = null;
    this.readyAt = performance.now() + delay;
    this.notify();
    this.wake(delay);
  }
  wake(delay=0) {
    clearTimeout(this.timer);
    if (!this.closed) this.timer = setTimeout(() => this.pump(), delay);
  }
  invalidate() {
    for (const r of this.records) r.dependencies = undefined;
    this.schedule(0);
  }
  prioritize(key) { this.priority = key; this.readyAt = 0; this.wake(); }
  retry(key) {
    const r = this.records.find(r => r.key === key);
    if (r?.error) { delete r.error; r.valid = false; this.planError = null; }
    this.prioritize(key);
  }
  prepare() {
    const next = this.snapshot(), old = this.previous;
    const changed = new Set([...Object.keys(old.files), ...Object.keys(next.files)].filter(p => old.files[p] !== next.files[p]));
    for (const p of new Set([...Object.keys(old.assetKeys), ...Object.keys(next.assetKeys)]))
      if (old.assetKeys[p] !== next.assetKeys[p]) changed.add(p);
    const all = old.entry !== next.entry || old.page !== next.page || old.fontVersion !== next.fontVersion || changed.has('/@laymesh/font-bindings.json');
    for (const r of this.records) {
      if (all || r.error || !r.dependencies || r.dependencies.some(p => changed.has(p))) r.valid = false;
      delete r.error;
    }
    this.previous = next;
    this.pending = false;
    if (this.records.every(r => r.valid)) this.planVersion = this.version;
    this.event('invalidated', {pages:this.records.filter(r => !r.valid).map(r => r.key)});
  }
  mergeDirectory(directory) {
    if(!directory.length)throw Error('文档没有可用页面');
    const old = new Map(this.records.map(r => [r.key, r]));
    return directory.map(p => ({...(old.get(p.id) || {key:p.id, valid:false}), name:p.name || p.id}));
  }
  enqueue(action) {
    const next = this.commits.then(action);
    this.commits = next.catch(() => {});
    return next;
  }
  pump() {
    if (this.closed || this.paused()) return;
    const delay = (this.readyAt || 0) - performance.now();
    if (delay > 0) { this.wake(delay); return; }
    if (this.pending) this.prepare();
    const preferred = this.records.find(r => r.key === this.priority) || this.current;
    const ordered = [preferred, this.current, ...this.records].filter(Boolean);
    // Let the first visible commit finish before initializing background WASM
    // instances. Once it has finished, all available page owners can run.
    const limit = this.expandedVersion === this.version ? this.concurrency() : 1;
    for (const target of ordered) {
      if (this.jobs.size >= limit) break;
      if (target.valid || target.error || this.jobs.has(target.key) || !this.canRun(target.key)) continue;
      const job = {version:this.version, revision:this.revision(), target, trace:this.trace};
      this.jobs.set(target.key, job);
      void this.run(job);
    }
    this.notify();
  }
  async run(job) {
    const {target, version, revision, trace} = job;
    const valid = () => !this.closed && version === this.version && revision === this.revision();
    const event = (type, detail={}) => trace?.events.push({...detail,type,at:performance.now(),page:target.key});
    event('queued');
    try {
      const result = await this.compile({...this.previous, mode:'preview', targetPageId:target.key,
        activePageId:this.current?.key, fallbackPageIndex:Math.max(0,this.records.indexOf(this.current)), version},
        plan => this.enqueue(() => {
          if (!valid()) return;
          this.records = this.mergeDirectory(plan.directory);
          this.planVersion = version;
          this.planError = null;
          event('directory', {directory:plan.directory.map(p => p.id)});
          this.notify();
        }), progress=>{
          event('worker-progress',{...progress,workerAt:progress.at});
          if(progress.phase==='attempt-end'){
            const ms=progress.attempt.computeMs || 0;
            this.totals.attempts++;this.totals.computeMs+=ms;job.computeMs=(job.computeMs || 0)+ms;
          }
        });
      event('received', {diagnostics:result.diagnostics, compileMs:result.compileMs});
      await this.enqueue(async () => {
        if (!valid()) { event('discarded'); return; }
        this.validate?.(result);
        const records = this.mergeDirectory(result.directory);
        for (const key of result.unchanged || []) {
          const r = records.find(r => r.key === key);
          if (r?.value) { r.valid = true; r.dependencies = result.unchangedDependencies?.[key] || r.dependencies; delete r.error; }
        }
        for (const page of result.pages) {
          const r = records.find(r => r.key === page.id);
          Object.assign(r, {value:page, dependencies:page.dependencies, fonts:page.fonts,
            requestedFonts:page.requestedFonts, warnings:page.warnings, valid:true});
          delete r.error;
        }
        event('commit-start');
        await this.update(result, records, version);
        if (!valid()) { event('discarded'); return; }
        this.records = records;
        this.planVersion = version;
        this.expandedVersion = version;
        this.commit?.(result);
        event('committed', {pages:result.pages.map(p => p.id)});
        this.notify();
      });
    } catch (error) {
      event('failed', {error:error.message, diagnostics:error.diagnostics});
      await this.enqueue(() => {
        if (!valid()) return;
        const r = this.records.find(r => r.key === target.key);
        if (r) {
          r.valid = false; r.error = error;
          if (error.dependencies) r.dependencies = [...new Set([this.previous.entry, ...(r.dependencies || []), ...error.dependencies])];
        }
        if (this.planVersion !== version) this.planError = error;
        this.expandedVersion = version;
        this.error?.(error, target.key);
      });
    } finally {
      if(!valid())this.totals.cancelledComputeMs+=job.computeMs || 0;
      this.jobs.delete(target.key);
      this.notify();
      this.wake();
    }
  }
  async ensurePlan({version=this.version, signal}={}) {
    this.readyAt = 0; this.wake();
    for (;;) {
      this.checkWait(version, signal);
      if (!this.pending && this.planVersion === version) return this.records;
      if (!this.pending && this.planError) throw this.planError;
      await this.wait(signal);
    }
  }
  checkWait(version, signal) {
    if (this.closed) throw Error('编辑器已关闭');
    if (signal?.aborted) throw Error('准备已取消');
    if (version != null && version !== this.version) throw Error('源码版本已更新，请重新放映');
    if (this.paused()) throw Error('请完成输入后重试');
  }
  wait(signal) {
    return new Promise(resolve => {
      const wake = () => { this.waiters.delete(wake); signal?.removeEventListener('abort',wake); resolve(); };
      this.waiters.add(wake); signal?.addEventListener('abort',wake,{once:true});
    });
  }
  async ensure(key, {version, signal}={}) {
    if (key != null) this.prioritize(key); else { this.readyAt = 0; this.wake(); }
    for (;;) {
      this.checkWait(version, signal);
      const target = key == null ? null : this.records.find(r => r.key === key);
      if (!this.pending) {
        if (key == null ? this.complete : target?.valid) return;
        if (key != null && !target && this.planVersion === this.version) throw Error('页面不存在: ' + key);
        const error = key == null ? this.failure : target?.error || this.planError;
        if (error) throw error;
      }
      await this.wait(signal);
    }
  }
  dispose() {
    this.closed = true;
    clearTimeout(this.timer);
    this.notify();
    this.listeners.clear();
  }
}
