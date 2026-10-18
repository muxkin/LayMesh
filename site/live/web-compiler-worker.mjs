import init, {
  CompileSession,
  remove_preview_asset,
  register_preview_font,
  register_preview_asset,
  describe_font,
  decode_image,
  clear_preview_dependencies,
  preview_dependencies,
} from "./compiler/laymesh_wasm.js";
let ready;
let compilerModule;
let session,sourceFiles={},sourceVersion=0,assetPaths=new Set();
const drawings=new Map(),fontFaces=new Map(),workerSession=Math.random().toString(36).slice(2);
const pageInputs=new Map(),inputVersions=new Map();let inputVersion=0,fontVersion=0;
function inputChanged(path){inputVersions.set(path,++inputVersion);if(/\.(?:ttf|otf|ttc|otc)$/i.test(path)||path==='/@laymesh/font-bindings.json')fontVersion++;}
let chain = Promise.resolve(),latestVersion=0;
function progress(data,phase,detail={}) { self.postMessage({id:data.id,type:'compile-progress',version:data.version,phase,at:performance.timeOrigin+performance.now(),wasmMemoryBytes:compilerModule?.memory.buffer.byteLength,...detail}); }
async function evaluateSession(data,run,diagnostics,phase){
  let milliseconds=200,yields=0;
  for(;;){
    if(data.mode==='preview'&&data.version<latestVersion)throw Error('Compilation superseded');
    session.set_time_slice(milliseconds);
    const attempt={phase,startedAt:performance.timeOrigin+performance.now(),sliceMs:milliseconds};
    diagnostics.attempts.push(attempt);
    progress(data,'attempt-start',{attempt:{...attempt}});
    try{const value=run();attempt.status='success';return {value,yields};}
    catch(error){
      let code;try{code=JSON.parse(String(error)).code;}catch{}
      attempt.status=code||String(error);
      attempt.computeMs=performance.timeOrigin+performance.now()-attempt.startedAt;
      if(code!=='E_YIELD')throw error;
      yields++;
      await new Promise(resolve=>setTimeout(resolve,0));
      // Completed queries survive a yield. Increase the slice when replaying
      // a large prefix costs more than one slice, guaranteeing forward progress.
      // Some semantic units cannot yield halfway through (font shaping,
      // geometry). Account for their actual duration when replaying, otherwise
      // a slow unit can force many fruitless retries or never fit a fixed cap.
      milliseconds=Math.max(milliseconds*2,attempt.computeMs*2);
    }finally{attempt.durationMs=performance.timeOrigin+performance.now()-attempt.startedAt;attempt.computeMs??=attempt.durationMs;attempt.metrics=JSON.parse(session.metrics());progress(data,'attempt-end',{attempt:{...attempt}});session.set_time_slice(0);}
  }
}
self.onmessage = ({ data }) => {
  if(data.type==='cancel'){latestVersion=Math.max(latestVersion,data.version||0);return;}
  if(data.mode==='preview')latestVersion=Math.max(latestVersion,data.version||0);
  const enqueuedAt=performance.timeOrigin+performance.now();
  chain = chain.then(async () => {
    const diagnostics={version:data.version,enqueuedAt,startedAt:performance.timeOrigin+performance.now(),attempts:[]};
    try {
      if(data.type==='retain'){
        for(const [key,previous] of drawings)if(!data.pages.includes(key)){previous.job.free();drawings.delete(key);pageInputs.delete(key);}
        return;
      }
      if (data.type === "init") {
        ready = init({ module_or_path: data.wasm });
        compilerModule = await ready;
        session=new CompileSession();
        if(data.profile)session.set_profiling(true);
        for (const f of data.fonts) register_preview_font(f.path, f.bytes);
        self.postMessage({ id: data.id, ready: true });
        return;
      }
      await ready;
      if (data.type === "fontInfo") {
        self.postMessage({
          id: data.id,
          faces: JSON.parse(describe_font(data.bytes)),
        });
        return;
      }
      if (data.type === "decodeImage") {
        self.postMessage({
          id: data.id,
          image: JSON.parse(decode_image(data.bytes, data.path)),
        });
        return;
      }
      const started = performance.now();
      progress(data,'input-start');
      clear_preview_dependencies();
      const assetChanges=data.assetChanges||{...Object.fromEntries([...assetPaths].filter(p=>!(p in data.assets)).map(p=>[p,null])),...data.assets};
      for (const [path, bytes] of Object.entries(assetChanges)) {
        inputChanged(path);
        if(bytes==null){remove_preview_asset(path);assetPaths.delete(path);}
        else {register_preview_asset(path, bytes);assetPaths.add(path);}
      }
      const changes=data.fileChanges||{};
      if(!data.fileChanges)for(const path of new Set([...Object.keys(sourceFiles),...Object.keys(data.files)]))if(sourceFiles[path]!==data.files[path])changes[path]=data.files[path]??null;
      session.apply_changes(++sourceVersion,JSON.stringify(changes));
      for(const [path,value] of Object.entries(changes)){inputChanged(path);if(value==null)delete sourceFiles[path];else sourceFiles[path]=value;}
      const inputMs=performance.now()-started;
      const pages = [],
        warnings = [],
        faces = new Map(),
        requestedFonts = new Map(),
        plan = null;
      try {
        const {value:metadataJSON,yields:planYields}=await evaluateSession(data,()=>session.metadata(data.entry),diagnostics,'directory');
        const metadata = JSON.parse(metadataJSON);
        session.set_active_page(data.activePageId||data.targetPageId||'');
        const planMs=performance.now()-started-inputMs;
        for(const [key,previous] of drawings)if(!metadata.pages.some(p=>p.id===key)){previous.job.free();drawings.delete(key);}
        for(const key of pageInputs.keys())if(!metadata.pages.some(p=>p.id===key))pageInputs.delete(key);
        const documentDependencies = [
          ...new Set([data.entry, ...JSON.parse(preview_dependencies())]),
        ];
        const directory = metadata.pages.filter(
          (_, i) => data.page == null || i + 1 === data.page,
        );
        if(!directory.length)throw Error('文档没有可用页面');
        self.postMessage({id:data.id,type:'compile-plan',version:data.version,directory,name:metadata.name});
        // A document name/order edit changes the directory, not page inputs.
        // Revalidate each page's actual reads and its visual descriptor, even
        // if its heavy draw job was evicted. Only the client's exact accepted
        // baseline can be reused; discarded/obsolete replies cannot pass here.
        const descriptor=p=>JSON.stringify({src:p.src,mediaKey:p.mediaKey,transition:p.transition&&{args:p.transition.args,file:p.transition.file}});
        const unchanged=data.mode==='preview'?directory.filter(p=>{const old=pageInputs.get(p.id);return old&&old.listId===data.knownPages?.[p.id]&&old.spec===descriptor(p)&&old.fontVersion===fontVersion&&old.reads.every(([path,version])=>(inputVersions.get(path)||0)===version);}).map(p=>p.id):[];
        const target = directory.find((p) => p.id === data.targetPageId) ||
          directory[Math.min(data.fallbackPageIndex || 0, directory.length - 1)];
        for (let i = 0; i < metadata.pages.length; i++) {
          if (data.page != null && i + 1 !== data.page) continue;
          if (data.mode === "preview" && metadata.pages[i] !== target) continue;
          if (unchanged.includes(metadata.pages[i].id)) continue;
          clear_preview_dependencies();
          const pageStarted=performance.now();
          const page = metadata.pages[i];
          const {value:job,yields}=await evaluateSession(data,()=>session.evaluate(page.id),diagnostics,'evaluate:'+page.id);
          const evaluateMs=performance.now()-pageStarted;
          progress(data,'drawing-start',{page:page.id});
          let draw;
          try {
            const previous=drawings.get(page.id),listId=`${page.id}@${workerSession}:${sourceVersion}`;
            let listUpdate;
            if(previous){draw=previous.job;listUpdate=JSON.parse(job.update_draw_job_incremental(draw,true));}
            else draw=job.draw_job_outlined();
            const drawMs=performance.now()-pageStarted-evaluateMs;
            progress(data,'serialization-start',{page:page.id});
            const partial=data.mode==='preview'&&previous&&data.knownPages?.[page.id]===previous.id;
            const list = JSON.parse(partial?draw.drawing_info():draw.display_list()),
              resources = Object.fromEntries(
                Object.keys(list.resources)
                  .filter((id) => /^(image|video|audio)\//.test(list.resources[id].mime)&&(!partial||!previous.resourceIds.has(id)))
                  .map((id) => [id, draw.resource_bytes(id)]),
              );
            drawings.delete(page.id);drawings.set(page.id,{job:draw,id:listId,fontIds:Object.keys(list.resources).filter(id=>list.resources[id].mime.startsWith('font/')),resourceIds:new Set(Object.keys(list.resources).filter(id=>/^(image|video|audio)\//.test(list.resources[id].mime)))});
            const inspected = JSON.parse(job.inspection());
            const dependencies=JSON.parse(preview_dependencies());
            pageInputs.set(page.id,{spec:descriptor(page),listId,fontVersion,reads:dependencies.map(path=>[path,inputVersions.get(path)||0])});
            const pageFaces = new Map();
            for (const r of inspected.requestedFonts || [])
              requestedFonts.set(JSON.stringify(r), r);
            warnings.push(...(inspected.warnings || []));
            for (const [id, r] of Object.entries(list.resources))
              if (r.mime.startsWith("font/")) {
                if(!fontFaces.has(id))fontFaces.set(id,JSON.parse(describe_font(draw.resource_bytes(id))));
                for (const face of fontFaces.get(id)) {
                  faces.set(face.postscriptName, face);
                  pageFaces.set(face.postscriptName, face);
                }
              }
            for(const [id,r] of Object.entries(list.resources))if(r.mime.startsWith("font/"))delete list.resources[id];
            if(listUpdate?.resources)for(const [id,r] of Object.entries(listUpdate.resources))if(r.mime.startsWith("font/"))delete listUpdate.resources[id];
            diagnostics.pages ||= [];
            diagnostics.pages.push({id:page.id,evaluateMs,drawMs,serializeMs:performance.now()-pageStarted-evaluateMs-drawMs});
            pages.push({
              id: page.id,
              name: page.name || page.id,
              list:partial?undefined:list,
              listId,
              listUpdate:partial?{base:previous.id,update:listUpdate}:undefined,
              resources,
              resourceDelta:!!partial,
              inspection: inspected.inspection,
              compileMetrics: {...JSON.parse(session.metrics()),cooperativeYields:yields+planYields,pipelineMs:{input:inputMs,plan:planMs,evaluate:evaluateMs,drawing:drawMs,metadata:performance.now()-pageStarted-evaluateMs-drawMs}},
              dependencies: [
                ...new Set([...documentDependencies, ...dependencies]),
              ],
              fonts: [...pageFaces.values()],
              requestedFonts: inspected.requestedFonts || [],
              warnings: inspected.warnings || [],
            });
          } finally {
            job.free();
          }
        }
        const liveFonts=new Set();
        for(const drawing of drawings.values())for(const id of drawing.fontIds)liveFonts.add(id);
        for(const id of fontFaces.keys())if(!liveFonts.has(id))fontFaces.delete(id);
        if (!pages.length&&!unchanged.length) throw Error("Selected page no longer exists");
        progress(data,'serialization-end');
        self.postMessage(
          {
            id: data.id,
            pages,
            directory,
            unchanged,
            unchangedDependencies:Object.fromEntries(unchanged.map(id=>[id,[...new Set([...documentDependencies,...pageInputs.get(id).reads.map(([path])=>path)])]])),
            name: metadata.name,
            version: data.version,
            fonts: [...faces.values()],
            requestedFonts: [...requestedFonts.values()],
            dependencies: [...new Set(pages.flatMap(p => p.dependencies))],
            warnings,
            diagnostics:{...diagnostics,finishedAt:performance.timeOrigin+performance.now(),inputMs,planMs},
            compileMs: performance.now() - started,
            wasmMemoryBytes: compilerModule.memory.buffer.byteLength,
          },
          pages.flatMap((p) => Object.values(p.resources).map((b) => b.buffer)),
        );
      } finally {
        plan?.free();
      }
    } catch (error) {
      self.postMessage({ id: data.id, error: String(error),diagnostics:{...diagnostics,finishedAt:performance.timeOrigin+performance.now()},
        dependencies: compilerModule ? JSON.parse(preview_dependencies()) : [] });
    }
  });
};
