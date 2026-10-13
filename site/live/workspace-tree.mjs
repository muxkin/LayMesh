import {
  createTree,
  syncDataLoaderFeature,
  selectionFeature,
  hotkeysCoreFeature,
  dragAndDropFeature,
  keyboardDragAndDropFeature,
  renamingFeature,
} from "@headless-tree/core";
import { uiIcon } from "./presentation-icons.mjs";
import { parentPath, fileName, within } from "./workspace-files.mjs";
// Thin DOM adapter: interaction/state comes from Headless Tree's core package.
export function workspaceTree(
  container,
  {
    entries,
    directories,
    open,
    selectDirectory,
    move,
    rename,
    importDrop,
    onError,
    menu,
    shortcut,
  },
) {
  let nodes = new Map(),
    queued = false,
    disposed = false;
  const rows = new Map();
  const root = "$root",
    groups = ["/project", "/@generated", "laymesh-library:"];
  const schedule = () => {
    if (!queued && !disposed) {
      queued = true;
      queueMicrotask(() => {
        queued = false;
        if (!disposed) render();
      });
    }
  };
  const tree = createTree({
    rootItemId: root,
    initialState: { expandedItems: [...groups], selectedItems: [] },
    setState: schedule,
    getItemName: (item) => item.getItemData()?.name || "",
    isItemFolder: (item) => !!item.getItemData()?.folder,
    dataLoader: {
      getItem: (id) => nodes.get(id),
      getChildren: (id) => nodes.get(id)?.children || [],
    },
    onPrimaryAction: (item) => {
      const n = item.getItemData();
      if (!n) return;
      if (!n.readonly) selectDirectory(n.folder ? n.path : parentPath(n.path));
      if (!n.folder) open(n.path).catch(onError);
    },
    canRename: (item) =>
      !item.getItemData()?.readonly && !item.getItemData()?.group,
    onRename: (item, name) => rename(item.getId(), name).catch(onError),
    canReorder: false,
    createForeignDragObject: (items) => ({
      format: "application/x-laymesh-workspace",
      data: JSON.stringify(items.map((i) => i.getId())),
      effectAllowed: "move",
      dropEffect: "move",
    }),
    openOnDropDelay: 650,
    canDrag: (items) =>
      items.length > 0 &&
      items.every((i) => !i.getItemData()?.readonly && !i.getItemData()?.group),
    canDrop: (items, target) => {
      const n = target.item.getItemData();
      return (
        !!n?.folder &&
        !n.readonly &&
        items.every((i) => !within(n.path, i.getId()))
      );
    },
    onDrop: async (items, target) => {
      await move(
        items.map((i) => i.getId()),
        target.item.getId(),
      ).catch(onError);
      // Paths are IDs here. Upstream restores the old dragged ID after drop;
      // restore DOM focus to the moved selection once that callback completes.
      setTimeout(() => {
        if (!disposed) {
          tree.getSelectedItems()[0]?.setFocused();
          tree.updateDomFocus();
        }
      }, 0);
    },
    canDragForeignDragObjectOver: (dt, target) =>
      [...dt.types].includes("Files") && !target.item.getItemData()?.readonly,
    canDropForeignDragObject: (dt, target) =>
      [...dt.types].includes("Files") && !target.item.getItemData()?.readonly,
    onDropForeignDragObject: (dt, target) =>
      importDrop(dt, target.item.getId()).catch(onError),
    features: [
      syncDataLoaderFeature,
      selectionFeature,
      hotkeysCoreFeature,
      dragAndDropFeature,
      keyboardDragAndDropFeature,
      renamingFeature,
    ],
  });
  function props(el, values) {
    for (const [key, value] of Object.entries(values)) {
      if (key === "ref") value?.(el);
      else if (key.startsWith("on"))
        el["on" + (key === "onChange" ? "input" : key.slice(2).toLowerCase())] =
          value;
      else if (key === "tabIndex" || key === "draggable" || key === "value")
        el[key] = value;
      else if (value == null) el.removeAttribute(key);
      else el.setAttribute(key, String(value));
    }
  }
  function render() {
    props(container, tree.getContainerProps("项目文件"));
    const visible = new Set();
    let previous = null;
    for (const item of tree.getItems()) {
      const id = item.getId(),
        n = nodes.get(id);
      if (!n) continue;
      visible.add(id);
      let row = rows.get(id);
      if (!row) {
        row = document.createElement("div");
        row.className = "workspace-tree-row";
        row.dataset.path = id;
        rows.set(id, row);
      }
      props(row, item.getProps());
      row.style.paddingLeft = 12 + item.getItemMeta().level * 16 + "px";
      row.dataset.folder = String(n.folder);
      row.dataset.group = String(!!n.group);
      row.dataset.readonly = String(!!n.readonly);
      row.classList.toggle("drop-target", item.isDragTarget());
      row.classList.toggle("tree-file", !n.folder);
      row.title = id;
      row.oncontextmenu = (e) => {
        e.preventDefault();
        e.stopPropagation();
        if (!item.isSelected()) tree.setSelectedItems([id]);
        item.setFocused();
        if (!n.readonly) selectDirectory(n.folder ? id : parentPath(id));
        menu(e, id);
      };
      if (item.isRenaming()) {
        let input = row.querySelector("input");
        if (!input) {
          row.replaceChildren();
          input = document.createElement("input");
          input.setAttribute("aria-label", "重命名 " + n.name);
          row.append(input);
          props(input, item.getRenameInputProps());
          input.select();
        } else {
          const p = item.getRenameInputProps();
          delete p.ref;
          props(input, p);
        }
      } else if (
        row.dataset.label !== n.name ||
        row.dataset.expanded !== String(item.isExpanded()) ||
        row.querySelector("input")
      ) {
        row.replaceChildren();
        const arrow = document.createElement("span");
        arrow.className = "tree-chevron";
        arrow.textContent = n.folder ? (item.isExpanded() ? "▾" : "▸") : "";
        row.append(
          arrow,
          uiIcon(
            n.folder
              ? "folder"
              : /\.(png|jpe?g|webp|svg)$/i.test(id)
                ? "image"
                : "page",
          ),
        );
        const label = document.createElement("span");
        label.textContent = n.name;
        row.append(label);
        row.dataset.label = n.name;
        row.dataset.expanded = String(item.isExpanded());
      }
      const desired = previous ? previous.nextSibling : container.firstChild;
      if (row !== desired) container.insertBefore(row, desired);
      previous = row;
    }
    for (const [id, row] of rows)
      if (!visible.has(id)) {
        tree.getItemInstance(id).registerElement(null);
        row.remove();
        rows.delete(id);
      }
  }
  function refresh() {
    nodes = new Map([[root, { children: [] }]]);
    for (const [i, id] of groups.entries())
      nodes.set(id, {
        path: id,
        name: ["项目文件", "生成资源 · 只读", "内置样式库 · 只读"][i],
        group: true,
        folder: true,
        readonly: i > 0,
        children: [],
      });
    const folder = (path, readonly = false) => {
      if (nodes.has(path)) return;
      if (!path) return;
      folder(parentPath(path), readonly);
      nodes.set(path, {
        path,
        name: fileName(path),
        folder: true,
        readonly,
        children: [],
      });
      nodes.get(parentPath(path))?.children.push(path);
    };
    for (const path of directories()) folder(path);
    for (const [path, e] of entries) {
      folder(parentPath(path), e.readonly);
      nodes.set(path, {
        path,
        name: fileName(path),
        folder: false,
        readonly: e.readonly,
        children: [],
      });
      nodes.get(parentPath(path))?.children.push(path);
    }
    nodes.get(root).children = groups.filter(
      (id) => id === "/project" || nodes.get(id).children.length,
    );
    for (const [id, n] of nodes)
      if (id !== root)
        n.children?.sort(
          (a, b) =>
            Number(nodes.get(b).folder) - Number(nodes.get(a).folder) ||
            nodes.get(a).name.localeCompare(nodes.get(b).name),
        );
    tree.setConfig((c) => ({
      ...c,
      state: {
        ...tree.getState(),
        selectedItems: tree
          .getState()
          .selectedItems.filter((id) => nodes.has(id)),
        focusedItem: nodes.has(tree.getState().focusedItem)
          ? tree.getState().focusedItem
          : null,
      },
    }));
    tree.rebuildTree();
    render();
  }
  tree.setMounted(true);
  refresh();
  const key = (e) => {
    if (e.isComposing || e.target.closest("input")) return;
    if (e.key === "ContextMenu" || (e.shiftKey && e.key === "F10")) {
      e.preventDefault();
      const item = tree.getFocusedItem();
      const row = item?.getElement() || container;
      const r = row.getBoundingClientRect();
      menu(
        { clientX: r.left + 24, clientY: r.bottom, target: row },
        item?.getId(),
      );
    } else
      shortcut?.(
        e,
        tree.getSelectedItems().map((i) => i.getId()),
      );
  };
  container.addEventListener("keydown", key);
  const scroll = (e) => {
    const r = container.getBoundingClientRect();
    if (e.clientY < r.top + 30) container.scrollTop -= 18;
    else if (e.clientY > r.bottom - 30) container.scrollTop += 18;
  };
  container.addEventListener("dragover", scroll);
  return {
    tree,
    refresh,
    snapshot() {
      const { selectedItems, expandedItems, focusedItem } = tree.getState();
      return {
        selectedItems: [...selectedItems],
        expandedItems: [...expandedItems],
        focusedItem,
        scrollTop: container.scrollTop,
      };
    },
    restore(state) {
      tree.setConfig((config) => ({
        ...config,
        state: {
          ...tree.getState(),
          selectedItems: state.selectedItems.filter((id) => nodes.has(id)),
          expandedItems: state.expandedItems.filter((id) => nodes.has(id)),
          focusedItem: nodes.has(state.focusedItem) ? state.focusedItem : null,
        },
      }));
      tree.rebuildTree();
      render();
      container.scrollTop = state.scrollTop;
    },
    selected: () => tree.getSelectedItems().map((i) => i.getId()),
    rename: (id) => tree.getItemInstance(id).startRenaming(),
    select(path) {
      let p = parentPath(path);
      while (nodes.has(p)) {
        tree.getItemInstance(p).expand();
        p = parentPath(p);
      }
      tree.setSelectedItems([path]);
      tree.getItemInstance(path).setFocused();
      render();
      rows.get(path)?.scrollIntoView({ block: "nearest" });
    },
    dispose() {
      disposed = true;
      tree.setMounted(false);
      tree.registerElement(null);
      container.removeEventListener("keydown", key);
      container.removeEventListener("dragover", scroll);
      container.replaceChildren();
    },
  };
}
