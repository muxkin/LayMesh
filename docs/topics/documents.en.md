# Multi-page documents

`.lay` defines an independent page. `.laydoc` orchestrates pages using the same Lay parser and language: variables, functions, conditions, loops and module imports. Each page retains its original `canvas` and remains independently renderable.

## document

`doc = document(name="Research report")` creates the one document in an orchestration entry. The default name is `Untitled`. `doc.pages` returns page references in insertion order.

## page

`cover = page(src="./pages/cover.lay", id="cover", name="Cover")` creates a lazy page reference. `src` is a local relative `.lay` path resolved against the calling file, including imported function definitions. IDs must be unique within a document and default to the file stem; names default to IDs. Read `cover.id`, `cover.name` and `cover.src`.

Pages have independent variables, units, layout DPI and styles. Their assets remain relative to the original page file. Repeated source references share one compiled scene within a full document compilation, with distinct IDs.

## document.add

```lay
doc = document(name="Report")
pages = [
    page(src="./pages/cover.lay", id="cover"),
    page(src="./pages/results.lay", id="results"),
]
for p in pages {
    doc.add(p)
}
```

Execution order determines page order. Reorder, add or remove list entries to manage pages. A document has 1–10,000 pages. Duplicate IDs, missing sources and page compilation errors are reported explicitly. Transitions are not implemented; passing `transition` is an error.

## Export and preview

```sh
laymesh validate examples/document/report.laydoc
laymesh inspect examples/document/report.laydoc --json
laymesh render examples/document/report.laydoc -o report.pdf
laymesh render examples/document/report.laydoc -o results.png --page 2
laymesh render examples/document/report.laydoc -o page.png --all-pages
```

Page numbers start at 1. PDF exports the entire document by default, preserving each page's physical size and vector content. Other formats require `--page N` or `--all-pages`; batch files are named `page-001.png`, `page-002.png`, etc. Whole-document PPTX is not implemented; selected-page and numbered PPTX exports retain existing editable objects and fallback diagnostics.

VS Code previews offer page selection, opening page sources, exporting the current page and exporting the document PDF. Web previews use the same Rust/WASM orchestration and render the selected page. Stable IDs preserve selection when pages are reordered; removing the selected page produces an explicit error.

```python
from laymesh import render_document

result = render_document("examples/document/report.laydoc", output="report.pdf")
result = render_document("examples/document/report.laydoc", output="results.png", page=2)
```

`DocumentResult` provides `output`, `page` and `preview_jpeg`. Display it directly in a Notebook or use `%laymesh report.laydoc --page 2`. Native CPU previews use 192 DPI, quality 95, 4:4:4 JPEG. `preview_page=2` changes only the preview selection while exporting the entire PDF.

Complete example: [report.laydoc](../../examples/document/report.laydoc).
