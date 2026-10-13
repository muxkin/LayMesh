import $ from "workspace-ui";
let sequence = 0;
// One upstream context-menu instance; LayMesh supplies commands and context only.
export function createContextMenu({
  root,
  resolve,
  onError = console.error,
  contains = (target) => root.contains(target),
}) {
  const trigger = document.createElement("span");
  trigger.id = "laymesh-menu-trigger-" + ++sequence;
  trigger.tabIndex = -1;
  trigger.style.cssText = "position:fixed;width:0;height:0;";
  document.body.append(trigger);
  const selector = "#" + trigger.id;
  let current = [],
    point,
    returnFocus,
    layer,
    afterClose,
    visible = false;
  const hide = () => $(trigger).contextMenu("hide");
  $.contextMenu({
    selector,
    trigger: "none",
    build: () => ({
      appendTo: layer || document.body,
      className: "laymesh-context-menu",
      animation: { duration: 0, show: "show", hide: "hide" },
      items: Object.fromEntries(
        current.map((item, i) => [
          "command" + i,
          item === "separator"
            ? "---------"
            : {
                name: item.name,
                disabled: !!item.disabled,
                callback: () => {
                  // Let the plugin finish hiding first. Commands may remove
                  // their dialog/presentation host, which otherwise detaches
                  // the menu before its delegated cleanup handler can run.
                  queueMicrotask(()=>{try{Promise.resolve(item.run?.()).catch(onError);}catch(e){onError(e);}});
                },
              },
        ]),
      ),
      position: (_opt) => {
        const menu = _opt.$menu;
        menu.css({
          position: "fixed",
          display: "block",
          left: Math.max(
            4,
            Math.min(point.x, innerWidth - menu.outerWidth() - 4),
          ),
          top: Math.max(
            4,
            Math.min(point.y, innerHeight - menu.outerHeight() - 4),
          ),
          zIndex: 2147483647,
        });
      },
      events: {
        activated: (options) => {
          // The upstream dismissal layer defaults to body. Keep it in the
          // menu's stacking/top layer so it cannot cover a presentation menu.
          options.$layer?.appendTo(layer || document.body);
        },
        show: () => {
          visible = true;
        },
        hide: () => {
          visible = false;
          afterClose?.();
          afterClose = null;
          if (returnFocus?.isConnected)
            returnFocus.focus({ preventScroll: true });
        },
      },
    }),
  });
  function show(event, items, options = {}) {
    hide();
    current = items.filter(Boolean);
    if (!current.length) return;
    point = { x: event.clientX, y: event.clientY };
    returnFocus = document.activeElement;
    afterClose = options.onClose;
    layer =
      event.target?.closest(
        "dialog[open],[popover]:popover-open,.presentation",
      ) ||
      document.fullscreenElement ||
      document.body;
    layer.append(trigger);
    $(trigger).contextMenu({ x: point.x, y: point.y });
  }
  function open(event) {
    if (
      event.target.closest(".context-menu-list,.context-menu-layer") ||
      !contains(event.target)
    )
      return;
    hide();
    const result = resolve(event.target, event);
    if (!result) return;
    event.preventDefault();
    event.stopImmediatePropagation();
    show(event, Array.isArray(result) ? result : result.items, result);
  }
  const keyboard = (event) => {
    if (event.isComposing) return;
    // The upstream menu owns keyboard navigation while visible. Forward directly
    // to its document handler before presentation/CodeMirror handlers see it.
    if (visible) {
      event.preventDefault();
      event.stopImmediatePropagation();
      $(document).trigger(
        $.Event("keydown", {
          target: trigger,
          key: event.key,
          keyCode: event.keyCode,
          which: event.which,
          shiftKey: event.shiftKey,
          ctrlKey: event.ctrlKey,
          metaKey: event.metaKey,
          altKey: event.altKey,
        }),
      );
      return;
    }
    if (
      event.key === "ContextMenu" ||
      (event.shiftKey && event.key === "F10")
    ) {
      const target = document.activeElement;
      if (!contains(target)) return;
      const r = target.getBoundingClientRect();
      open({
        target,
        clientX: r.left + 20,
        clientY: r.bottom,
        preventDefault: () => event.preventDefault(),
        stopImmediatePropagation: () => event.stopImmediatePropagation(),
      });
    }
  };
  document.addEventListener("contextmenu", open, true);
  document.addEventListener("keydown", keyboard, true);
  return {
    show,
    hide,
    dispose() {
      hide();
      $.contextMenu("destroy", selector);
      trigger.remove();
      document.removeEventListener("contextmenu", open, true);
      document.removeEventListener("keydown", keyboard, true);
    },
  };
}
export async function writeClipboard(text) {
  if (!navigator.clipboard?.writeText)
    throw Error("浏览器未开放剪贴板，请使用 Ctrl / ⌘ + C 复制选中的内容。");
  try {
    await navigator.clipboard.writeText(text);
  } catch {
    throw Error("无法访问剪贴板，请使用 Ctrl / ⌘ + C。");
  }
}
export async function readClipboard() {
  if (!navigator.clipboard?.readText)
    throw Error("浏览器未开放剪贴板，请使用 Ctrl / ⌘ + V 粘贴。");
  try {
    return await navigator.clipboard.readText();
  } catch {
    throw Error("无法访问剪贴板，请使用 Ctrl / ⌘ + V。");
  }
}
export function inputMenu(input) {
  const textSelection = input.selectionStart != null;
  const a = input.selectionStart ?? 0,
    b = input.selectionEnd ?? input.value?.length ?? 0,
    selection = input.value?.slice(a, b) || "",
    editable = !input.readOnly && !input.disabled;
  const replace = (text) => {
    input.focus();
    if (textSelection) {
      input.setSelectionRange(a, b);
      input.setRangeText(text, a, b, "end");
    } else input.value = text;
    input.dispatchEvent(new Event("input", { bubbles: true }));
  };
  return [
    {
      name: "剪切",
      disabled: !editable || !selection,
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
      disabled: !editable,
      run: async () => replace(await readClipboard()),
    },
    "separator",
    {
      name: "全选",
      run: () => {
        input.focus();
        input.select();
      },
    },
  ];
}
