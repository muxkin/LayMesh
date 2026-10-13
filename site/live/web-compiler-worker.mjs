import init, {
  prepare_render,
  prepare_presentation,
  register_preview_font,
  register_preview_asset,
  clear_preview_assets,
  describe_font,
  decode_image,
  clear_preview_dependencies,
  preview_dependencies,
} from "./compiler/laymesh_wasm.js";
let ready;
let compilerModule;
let chain = Promise.resolve();
self.onmessage = ({ data }) => {
  chain = chain.then(async () => {
    try {
      if (data.type === "init") {
        ready = init({ module_or_path: data.wasm });
        compilerModule = await ready;
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
      clear_preview_assets();
      clear_preview_dependencies();
      for (const [path, bytes] of Object.entries(data.assets))
        register_preview_asset(path, bytes);
      const encoded = JSON.stringify(data.files),
        pages = [],
        warnings = [],
        faces = new Map(),
        requestedFonts = new Map(),
        plan = data.entry.endsWith(".laydoc")
          ? prepare_presentation(data.files[data.entry], data.entry, encoded)
          : null;
      try {
        const metadata = plan
          ? JSON.parse(plan.metadata())
          : { pages: [{ id: "page", name: data.entry.split("/").at(-1) }] };
        for (let i = 0; i < metadata.pages.length; i++) {
          if (data.page != null && i + 1 !== data.page) continue;
          const page = metadata.pages[i],
            job = plan
              ? plan.page(i, encoded)
              : prepare_render(data.files[data.entry], data.entry, encoded);
          let draw;
          try {
            draw = job.draw_job();
            const list = JSON.parse(draw.display_list()),
              resources = Object.fromEntries(
                Object.keys(list.resources)
                  .filter((id) => list.resources[id].mime.startsWith("image/"))
                  .map((id) => [id, draw.resource_bytes(id)]),
              );
            const inspected = JSON.parse(job.inspection());
            for (const r of inspected.requestedFonts || [])
              requestedFonts.set(JSON.stringify(r), r);
            warnings.push(...(inspected.warnings || []));
            for (const [id, r] of Object.entries(list.resources))
              if (!r.mime.startsWith("image/")) {
                for (const face of JSON.parse(
                  describe_font(draw.resource_bytes(id)),
                ))
                  faces.set(face.postscriptName, face);
              }
            pages.push({
              id: page.id,
              name: page.name || page.id,
              list,
              svg: job.export_svg(),
              resources,
              inspection: inspected.inspection,
            });
          } finally {
            draw?.free();
            job.free();
          }
        }
        if (!pages.length) throw Error("Selected page no longer exists");
        self.postMessage(
          {
            id: data.id,
            pages,
            fonts: [...faces.values()],
            requestedFonts: [...requestedFonts.values()],
            dependencies: JSON.parse(preview_dependencies()),
            warnings,
            compileMs: performance.now() - started,
            wasmMemoryBytes: compilerModule.memory.buffer.byteLength,
          },
          pages.flatMap((p) => Object.values(p.resources).map((b) => b.buffer)),
        );
      } finally {
        plan?.free();
      }
    } catch (error) {
      self.postMessage({ id: data.id, error: String(error) });
    }
  });
};
