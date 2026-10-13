// Keep the full catalog in editable documents; playback only needs its pages.
export function selectPageTransitions(data, pages) {
  const source = new TextDecoder().decode(data);
  const marker = "export const transitions = ";
  const offset = source.indexOf(marker);
  if (offset < 0) throw Error("Invalid transition catalog");
  const catalog = JSON.parse(
    source
      .slice(offset + marker.length)
      .trim()
      .replace(/;$/, ""),
  );
  const selected = {};
  for (const page of pages) {
    const name = page.list.pageTransition?.name;
    if (!name) continue;
    if (!Object.hasOwn(catalog, name))
      throw Error("Missing transition: " + name);
    selected[name] = catalog[name];
  }
  return new TextEncoder().encode(
    "export const transitions = " + JSON.stringify(selected) + ";\n",
  );
}
