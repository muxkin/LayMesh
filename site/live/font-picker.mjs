import { displayFont, fontSearch } from "./font-names.mjs";
let sequence = 0;
export function fontPicker({
  dialog,
  label,
  value,
  choices,
  onSelect,
  placeholder,
}) {
  const box = document.createElement("div");
  box.className = "font-picker";
  const input = document.createElement("input"),
    clear = document.createElement("button"),
    list = document.createElement("div");
  input.type = "text";
  input.setAttribute("role", "combobox");
  input.setAttribute("aria-label", label);
  input.setAttribute("aria-autocomplete", "list");
  input.placeholder = placeholder || "搜索中文或英文字体名…";
  list.className = "font-options";
  list.id = "font-options-" + ++sequence;
  list.setAttribute("role", "listbox");
  list.hidden = true;
  dialog.append(list);
  input.setAttribute("aria-controls", list.id);
  clear.type = "button";
  clear.textContent = "×";
  clear.setAttribute("aria-label", "清除 " + label);
  box.append(input, clear);
  let options = choices,
    selected = value || "",
    filtered = [],
    active = -1,
    composing = false;
  let pendingOptions;
  const id = (f) => f.postscriptName || f.path + "#" + (f.fontIndex || 0);
  const name = (f) => {
    const d = displayFont(f);
    return d.family + " · " + d.style;
  };
  const initial = options.find((f) => id(f) === selected);
  input.value = initial ? name(initial) : "";
  const locate = () => {
    const r = input.getBoundingClientRect(),
      room = innerHeight - r.bottom - 12,
      above = room < 160;
    Object.assign(list.style, {
      left: Math.max(8, Math.min(r.left, innerWidth - 330)) + "px",
      width: Math.max(280, r.width) + "px",
      maxHeight: Math.max(90, Math.min(280, above ? r.top - 12 : room)) + "px",
      top: above ? "auto" : r.bottom + 4 + "px",
      bottom: above ? innerHeight - r.top + 4 + "px" : "auto",
    });
  };
  const hide = () => {
    list.hidden = true;
    input.setAttribute("aria-expanded", "false");
    input.removeAttribute("aria-activedescendant");
    if (pendingOptions) {
      options = pendingOptions;
      pendingOptions = null;
    }
  };
  const choose = (f) => {
    selected = f ? id(f) : "";
    input.value = f ? name(f) : "";
    hide();
    onSelect(selected, f);
  };
  const spacer = document.createElement("div");
  spacer.style.position = "relative";
  list.append(spacer);
  const renderOptions = () => {
    spacer.style.height = filtered.length * 54 + "px";
    spacer.replaceChildren();
    if (!filtered.length) {
      const empty = document.createElement("p");
      empty.textContent = "没有匹配字体";
      spacer.style.height = "48px";
      spacer.append(empty);
      return;
    }
    const first = Math.max(0, Math.floor(list.scrollTop / 54) - 2),
      last = Math.min(
        filtered.length,
        first + Math.ceil((list.clientHeight || 280) / 54) + 5,
      );
    for (let i = first; i < last; i++) {
      const f = filtered[i],
        row = document.createElement("div");
      row.className = "font-option";
      row.id = list.id + "-" + i;
      row.style.cssText =
        "position:absolute;left:0;right:0;top:" +
        i * 54 +
        "px;height:54px;box-sizing:border-box";
      row.setAttribute("role", "option");
      row.setAttribute("aria-selected", String(i === active));
      row.setAttribute("aria-posinset", String(i + 1));
      row.setAttribute("aria-setsize", String(filtered.length));
      const title = document.createElement("strong"),
        detail = document.createElement("small");
      title.textContent = name(f);
      detail.textContent = [f.family, f.postscriptName]
        .filter(Boolean)
        .join(" · ");
      row.append(title, detail);
      row.onpointerdown = (e) => {
        e.preventDefault();
        choose(f);
        input.focus();
      };
      spacer.append(row);
    }
  };
  list.onscroll = renderOptions;
  let lastQuery;
  const draw = () => {
    const query = input.value.normalize("NFKC").toLocaleLowerCase().trim();
    filtered = options.filter((f) => !query || fontSearch(f).includes(query));
    active = Math.min(active, filtered.length - 1);
    if (query !== lastQuery) {
      list.scrollTop = 0;
      lastQuery = query;
    }
    list.hidden = false;
    input.setAttribute("aria-expanded", "true");
    locate();
    if (active >= 0) {
      input.setAttribute("aria-activedescendant", list.id + "-" + active);
      const top = active * 54;
      if (top < list.scrollTop) list.scrollTop = top;
      else if (top + 54 > list.scrollTop + (list.clientHeight || 280))
        list.scrollTop = top + 54 - (list.clientHeight || 280);
    }
    renderOptions();
  };
  input.onfocus = () => {
    input.select();
    active = -1;
  };
  input.onclick = () => {
    if (pendingOptions) {
      options = pendingOptions;
      pendingOptions = null;
    }
    input.value = "";
    draw();
  };
  input.oninput = () => {
    if (!composing) {
      if (pendingOptions) {
        options = pendingOptions;
        pendingOptions = null;
      }
      active = -1;
      draw();
    }
  };
  input.addEventListener("compositionstart", () => {
    composing = true;
  });
  input.addEventListener("compositionend", () => {
    composing = false;
    if (pendingOptions) {
      options = pendingOptions;
      pendingOptions = null;
    }
    active = -1;
    draw();
  });
  input.onkeydown = (e) => {
    if (composing || e.isComposing) return;
    if (["ArrowDown", "ArrowUp"].includes(e.key)) {
      e.preventDefault();
      if (list.hidden) {
        input.value = "";
        active = -1;
      }
      active = Math.max(
        0,
        Math.min(
          filtered.length - 1,
          active + (e.key === "ArrowDown" ? 1 : -1),
        ),
      );
      draw();
    } else if (e.key === "Enter" && !list.hidden) {
      e.preventDefault();
      e.stopPropagation();
      if (filtered[active >= 0 ? active : 0])
        choose(filtered[active >= 0 ? active : 0]);
    } else if (e.key === "Escape" && !list.hidden) {
      e.preventDefault();
      e.stopPropagation();
      hide();
    } else if (e.key === "Tab") hide();
  };
  input.onblur = () =>
    setTimeout(() => {
      hide();
      input.value = options.find((f) => id(f) === selected)
        ? name(options.find((f) => id(f) === selected))
        : "";
    }, 0);
  clear.onclick = () => {
    choose(null);
    input.focus();
  };
  return {
    box,
    input,
    update(next) {
      if (next === options) return;
      // Keep the row under the pointer and the keyboard selection stable
      // while background name discovery changes the catalog's sort order.
      if (!list.hidden || composing) {
        pendingOptions = next;
        // A search with no results should still discover newly read aliases.
        if (!composing && !filtered.length) {
          options = next;
          pendingOptions = null;
          draw();
        }
        return;
      }
      options = next;
      if (!list.hidden && !composing) draw();
      else if (document.activeElement !== input)
        input.value = options.find((f) => id(f) === selected)
          ? name(options.find((f) => id(f) === selected))
          : "";
    },
    dispose() {
      list.remove();
    },
  };
}
