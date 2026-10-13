import {
  undo,
  redo,
  undoDepth,
  redoDepth,
  selectAll,
} from "@codemirror/commands";
import { openSearchPanel } from "@codemirror/search";
import { writeClipboard, readClipboard } from "./workspace-menu.mjs";
export function editorMenu(view, event, readonly) {
  const range = view.state.selection.main,
    pos = view.posAtCoords({ x: event.clientX, y: event.clientY });
  if (pos != null && (pos < range.from || pos > range.to))
    view.dispatch({ selection: { anchor: pos } });
  const state = view.state,
    selection = state.sliceDoc(
      state.selection.main.from,
      state.selection.main.to,
    ),
    revision = state.doc;
  const replace = (text) => {
    if (view.state.doc !== revision) throw Error("文本已改变，请重新粘贴");
    view.dispatch(view.state.replaceSelection(text));
    view.focus();
  };
  const search = (replaceMode) => {
    openSearchPanel(view);
    if (replaceMode) view.dom.querySelector("[name=replace]")?.focus();
  };
  return [
    {
      name: "撤销",
      disabled: readonly || !undoDepth(state),
      run: () => undo(view),
    },
    {
      name: "重做",
      disabled: readonly || !redoDepth(state),
      run: () => redo(view),
    },
    "separator",
    {
      name: "剪切",
      disabled: readonly || !selection,
      run: async () => {
        await writeClipboard(selection);
        replace("");
      },
    },
    {
      name: "复制",
      disabled: !selection,
      run: () => writeClipboard(selection),
    },
    {
      name: "粘贴",
      disabled: readonly,
      run: async () => replace(await readClipboard()),
    },
    { name: "全选", run: () => selectAll(view) },
    "separator",
    { name: "查找", run: () => search(false) },
    { name: "替换", disabled: readonly, run: () => search(true) },
    {
      name: "跳转定义",
      run: () => {
        view.focus();
        view.contentDOM.dispatchEvent(
          new KeyboardEvent("keydown", {
            key: "F12",
            code: "F12",
            bubbles: true,
            cancelable: true,
          }),
        );
      },
    },
  ];
}
