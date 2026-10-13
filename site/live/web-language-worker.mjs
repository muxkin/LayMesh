import init, {
  language_query,
  color_convert,
  color_presentations,
} from "./compiler/laymesh_wasm.js";
let ready;
self.onmessage = async ({ data }) => {
  if (data.type === "init") {
    ready = init({ module_or_path: data.wasm });
    return;
  }
  try {
    await ready;
    self.postMessage({
      id: data.id,
      result: JSON.parse(
        data.method === "colorConvert"
          ? color_convert(JSON.stringify(data.params))
          : data.method === "colorPresentations"
            ? color_presentations(
                JSON.stringify(data.files),
                data.file,
                data.params.from,
                data.params.to,
                JSON.stringify(data.params.rgba),
              )
            : language_query(
                JSON.stringify(data.files),
                data.file,
                data.method,
                data.offset || 0,
                data.locale || "zh",
              ),
      ),
    });
  } catch (error) {
    self.postMessage({ id: data.id, error: String(error) });
  }
};
