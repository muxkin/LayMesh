# LayMesh language support

Edit `.lay` and `.lcss` with completion, parameter hover, signature help, definitions, diagnostics and migration fixes. The server shares the documentation editor's static analyzer and never executes figure code.

Build from the repository root with `python scripts/build-editors.py`. Package with `python scripts/package-editor.py --output laymesh-language.vsix`; then use VS Code's **Extensions: Install from VSIX** command. No marketplace publication is required.

Keyboard: Ctrl+Space requests completion, Ctrl+Shift+Space requests a signature, and F12 goes to a definition. Full signatures open on request rather than on typed punctuation; hover over a function or parameter for documentation. On macOS use the corresponding editor key bindings.

Geometry defaults to mm. Bare typography, border/line widths and dash lengths use pt. See the bilingual documentation for LCSS, formulas and migration from older parameter names.

Document library functions with adjacent `##` comments and `@param`, `@returns`, `@example`, and `@see`; document exported variables with `@type`. Signatures and defaults come from declarations. Imports and aliases retain their documentation, and source or dependency changes refresh help without executing code. See [library documentation](../../docs/topics/library-docs.en.md) and the [Chinese guide](../../docs/topics/library-docs.zh-CN.md).

The website uses Tab to accept completion and Enter to insert a newline. This extension does not override VS Code key bindings. For the same Enter behavior, use language-specific settings:

```json
{
  "[laymesh]": { "editor.acceptSuggestionOnEnter": "off" },
  "[lcss]": { "editor.acceptSuggestionOnEnter": "off" }
}
```

## Help language and other LSP clients

`laymesh.language` accepts `auto` (default), `zh-CN`, or `en`. In VS Code, `auto` follows the editor display language. Changes update subsequent help and documentation warnings while preserving unsaved sources. The server never reads the host OS language to choose documentation.

Other IDEs can launch `laymesh lsp --stdio` from the repository root. Send `initialize.locale` for UI language or `initializationOptions: {"language":"zh-CN"}` for a startup preference. Clients supporting `workspace/configuration` return the `laymesh` section; others can push `workspace/didChangeConfiguration` with `{"settings":{"laymesh":{"language":"en"}}}`. A valid configuration setting wins over initialization options; `auto` uses the initial client locale. Missing or unsupported UI languages default to English. See the [LSP setup guide](../../docs/topics/editors.en.md#lsp-integration-in-other-ides).

Completion, hover and signatures negotiate Markdown or plain text per client capability. Signature parameter labels use ranges only when the client advertises support.

Library comments may use `## @lang zh-CN` and `## @lang en` sections. Missing fields fall back to untagged text, English, then the first authored version. Types, names, default values and inserted code are not translated. Existing untagged comments remain valid. See [multilingual documentation](../../docs/topics/library-docs.en.md#multilingual-documentation).

提示默认跟随界面语言；`laymesh.language` 可手动选择中文或英文。其他 IDE 使用相同的标准 LSP 服务，通过初始化选项或配置通知设置语言。库注释支持 `@lang` 分段，缺少译文时回退原文；详见[中文编辑器指南](../../docs/topics/editors.zh-CN.md)。
