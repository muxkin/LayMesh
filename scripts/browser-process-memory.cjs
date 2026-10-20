// Linux whole-process-tree PSS and per-client DRM accounting; GPU PSS is not VRAM.
const fs = require('node:fs');
const path = require('node:path');
const protocol = 'proc-ppid-v2';

function driverMemory(pid, procRoot) {
  const clients = new Map();
  let descriptors;
  try { descriptors = fs.readdirSync(path.join(procRoot, String(pid), 'fdinfo')); }
  catch { return []; }
  for (const fd of descriptors) {
    try {
      const raw = fs.readFileSync(path.join(procRoot, String(pid), 'fdinfo', fd), 'utf8');
      const fields = Object.fromEntries([...raw.matchAll(/^(drm-[\w-]+):\s*(.+)$/gm)].map(m => [m[1], m[2]]));
      if (fields['drm-client-id']) clients.set(fields['drm-pdev'] + '/' + fields['drm-client-id'], fields);
    } catch { /* An fd may close during a sample; continue with the others. */ }
  }
  return [...clients.values()];
}

function memory(pid, procRoot = '/proc') {
  const started = Date.now(), children = new Map();
  // /task/<main-thread>/children omits processes forked by another thread.
  // PPid in process status includes those descendants without double-counting threads.
  for (const entry of fs.readdirSync(procRoot)) {
    if (!/^\d+$/.test(entry)) continue;
    try {
      const status = fs.readFileSync(path.join(procRoot, entry, 'status'), 'utf8');
      const parent = Number(status.match(/^PPid:\s*(\d+)/m)?.[1]);
      const group = Number(status.match(/^Tgid:\s*(\d+)/m)?.[1] ?? entry);
      if (group !== Number(entry) || !Number.isFinite(parent)) continue;
      if (!children.has(parent)) children.set(parent, []);
      children.get(parent).push(Number(entry));
    } catch { /* Processes can exit while enumerating /proc. */ }
  }
  const pending = [pid], visited = new Set(), rows = [], unreadable = [];
  for (let i = 0; i < pending.length; i++) {
    const id = pending[i];
    if (visited.has(id)) continue;
    visited.add(id);
    pending.push(...(children.get(id) || []));
    try {
      const stat = fs.readFileSync(path.join(procRoot, String(id), 'smaps_rollup'), 'utf8');
      const cmd = fs.readFileSync(path.join(procRoot, String(id), 'cmdline'), 'utf8').replaceAll('\0', ' ');
      rows.push({pid: id, rss: Number(stat.match(/^Rss:\s+(\d+)/m)?.[1] || 0) * 1024,
        pss: Number(stat.match(/^Pss:\s+(\d+)/m)?.[1] || 0) * 1024, gpu: cmd.includes('--type=gpu-process')});
    } catch { unreadable.push(id); }
  }
  const drm = new Map();
  for (const row of rows) for (const client of driverMemory(row.pid, procRoot))
    drm.set(client['drm-pdev'] + '/' + client['drm-client-id'], client);
  return {protocol, at: started, sampleMs: Date.now() - started,
    rss: rows.reduce((n, r) => n + r.rss, 0), pss: rows.reduce((n, r) => n + r.pss, 0),
    gpuProcessPss: rows.some(r => r.gpu) ? rows.filter(r => r.gpu).reduce((n, r) => n + r.pss, 0) : null,
    drm: [...drm.values()], processes: rows.length, discoveredProcesses: visited.size, unreadable};
}
memory.protocol = protocol;
module.exports = memory;
