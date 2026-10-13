// Workspace-only file transactions. The language service supplies semantic references.
export const parentPath = (p) => p.slice(0, p.lastIndexOf("/"));
export const fileName = (p) => p.split("/").at(-1);
export const within = (path, root) =>
  path === root || path.startsWith(root + "/");
export function projectPath(path) {
  if (
    typeof path !== "string" ||
    !path.startsWith("/project/") ||
    /[\\\0]/.test(path)
  )
    throw Error("路径必须位于项目目录内");
  const parts = [];
  for (const p of path.split("/")) {
    if (!p || p === ".") continue;
    if (p === "..") parts.pop();
    else parts.push(p);
  }
  const result = "/" + parts.join("/");
  if (!result.startsWith("/project/")) throw Error("路径不能越出项目目录");
  return result;
}
export function validateName(name) {
  name = name.trim();
  if (
    !name ||
    name === "." ||
    name === ".." ||
    /[\\/:*?"<>|\x00-\x1f]/.test(name) ||
    /[. ]$/.test(name)
  )
    throw Error("请输入有效文件名，不含路径分隔符");
  return name;
}
export function relativePath(from, to) {
  const a = parentPath(from).split("/"),
    b = to.split("/");
  while (a.length && b.length && a[0] === b[0]) {
    a.shift();
    b.shift();
  }
  return [...a.map(() => ".."), ...b].join("/") || fileName(to);
}
export const topPaths = (paths) =>
  [...new Set(paths)].filter(
    (p) => !paths.some((q) => q !== p && within(p, q)),
  );
export function relocate(snapshot, mapping, references, { copy = false } = {}) {
  const pairs = Object.entries(mapping),
    map = (p) => {
      const pair = pairs.find(([a]) => within(p, a));
      return pair ? pair[1] + p.slice(pair[0].length) : p;
    };
  const state = {
    ...snapshot,
    entries: new Map([...snapshot.entries].map(([p, e]) => [p, { ...e }])),
    files: { ...snapshot.files },
    directories: new Set(snapshot.directories),
    project: structuredClone(snapshot.project),
    fonts: snapshot.fonts.map((f) => ({ ...f })),
    requirements: structuredClone(snapshot.requirements),
  };
  const affected = (p) => pairs.some(([a]) => within(p, a));
  for (const [from, to] of pairs) {
    projectPath(from);
    projectPath(to);
    if (from === to) continue;
    if (within(to, from)) throw Error("不能移入自身或子目录");
    if (state.entries.has(to) || state.directories.has(to))
      throw Error("目标已存在：" + to);
  }
  const pending = [];
  for (const [p, e] of snapshot.entries)
    if (affected(p)) {
      if (e.readonly) throw Error("只读资源不能移动");
      const q = map(p);
      pending.push([q, { ...e, path: q }]);
      if (!copy) {
        state.entries.delete(p);
        delete state.files[p];
      }
      if (snapshot.files[p] != null) state.files[q] = snapshot.files[p];
    }
  for (const [q, e] of pending) state.entries.set(q, e);
  for (const p of snapshot.directories)
    if (affected(p)) {
      if (!copy) state.directories.delete(p);
      state.directories.add(map(p));
    }
  for (const [, to] of pairs) {
    let p = parentPath(to);
    while (p.startsWith("/project/")) {
      state.directories.add(p);
      p = parentPath(p);
    }
  }
  const issues = [],
    changes = new Map();
  for (const ref of references) {
    if (copy && !affected(ref.uri)) continue;
    if (ref.dynamic) {
      issues.push(ref);
      continue;
    }
    const source = map(ref.uri),
      target = map(ref.target);
    if (source === ref.uri && (copy || target === ref.target)) continue;
    const path =
      (ref.path.startsWith("/") ? target : relativePath(source, target)) +
      (ref.suffix || "");
    const insert =
      ref.encoding === "css-url"
        ? "url(" + JSON.stringify(path) + ")"
        : JSON.stringify(path);
    const list = changes.get(source) || [];
    list.push({ from: ref.from, to: ref.to, insert });
    changes.set(source, list);
  }
  for (const [p, edits] of changes) {
    let text = state.files[p];
    if (text == null) continue;
    for (const edit of edits.sort((a, b) => b.from - a.from))
      text = text.slice(0, edit.from) + edit.insert + text.slice(edit.to);
    state.files[p] = text;
  }
  if (!copy) {
    state.project.entry = map(state.project.entry);
    state.project.dependencies = (state.project.dependencies || []).map(map);
    state.fonts = state.fonts.map((f) => ({
      ...f,
      path: f.path ? map(f.path) : f.path,
    }));
    for (const list of Object.values(state.requirements))
      if (Array.isArray(list))
        for (const f of list) if (f.path) f.path = map(f.path);
  }
  state.project.directories = [...state.directories].sort();
  return { state, issues, map, changes };
}
export async function droppedFiles(transfer) {
  // Capture entries before the drop event returns: browser drag stores become protected.
  const items = [...(transfer.items || [])],
    fallback = [...(transfer.files || [])];
  const roots = items.map((item) => item.webkitGetAsEntry?.()).filter(Boolean);
  if (!roots.length)
    return fallback.map((file) => ({
      file,
      path: file.webkitRelativePath || file.name,
    }));
  const files = [];
  async function visit(entry, prefix = "") {
    const path = prefix + entry.name;
    if (entry.isFile) {
      const file = await new Promise((ok, no) => entry.file(ok, no));
      files.push({ file, path });
    } else if (entry.isDirectory) {
      files.push({ path, directory: true });
      const reader = entry.createReader();
      let batch;
      do {
        batch = await new Promise((ok, no) => reader.readEntries(ok, no));
        for (const child of batch) await visit(child, path + "/");
      } while (batch.length);
    }
  }
  for (const entry of roots) await visit(entry);
  return files;
}
