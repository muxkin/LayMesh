import assert from "node:assert/strict";
import {
  relocate,
  projectPath,
  validateName,
} from "../site/live/workspace-files.mjs";
const text = 'doc.add(page(src="pages/a.lay"))',
  source = 'p=canvas(stylesheet="../styles/main.lcss")';
const snapshot = {
  entries: new Map([
    ["/project/doc.laydoc", { path: "/project/doc.laydoc" }],
    ["/project/pages/a.lay", { path: "/project/pages/a.lay" }],
  ]),
  files: { "/project/doc.laydoc": text, "/project/pages/a.lay": source },
  directories: new Set(["/project/pages", "/project/styles"]),
  project: { entry: "/project/doc.laydoc" },
  fonts: [],
  requirements: {},
};
const refs = [
  {
    uri: "/project/doc.laydoc",
    from: text.indexOf('"'),
    to: text.lastIndexOf('"') + 1,
    path: "pages/a.lay",
    target: "/project/pages/a.lay",
    encoding: "string",
  },
  {
    uri: "/project/pages/a.lay",
    from: source.indexOf('"'),
    to: source.lastIndexOf('"') + 1,
    path: "../styles/main.lcss",
    target: "/project/styles/main.lcss",
    encoding: "string",
  },
];
const moved = relocate(
  snapshot,
  { "/project/pages/a.lay": "/project/a.lay" },
  refs,
).state;
assert.equal(moved.files["/project/doc.laydoc"], 'doc.add(page(src="a.lay"))');
assert.equal(
  moved.files["/project/a.lay"],
  'p=canvas(stylesheet="styles/main.lcss")',
);
assert(!moved.entries.has("/project/pages/a.lay"));
assert(snapshot.entries.has("/project/pages/a.lay"));
const copied = relocate(snapshot, { "/project/pages": "/project/copy" }, refs, {
  copy: true,
}).state;
assert.equal(copied.files["/project/doc.laydoc"], text);
assert.equal(copied.files["/project/copy/a.lay"], source);
assert(copied.entries.has("/project/pages/a.lay"));
assert.throws(() =>
  relocate(snapshot, { "/project/pages": "/project/pages/child" }, refs),
);
assert.throws(() => projectPath("/project/../../escape"));
assert.throws(() => validateName("../bad"));
console.log(
  "workspace relocation, copy, relative references and boundary checks passed",
);
