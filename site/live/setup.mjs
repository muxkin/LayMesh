import {EditorState} from '@codemirror/state';
import {lineNumbers,highlightActiveLineGutter,highlightSpecialChars,drawSelection,dropCursor,rectangularSelection,crosshairCursor,highlightActiveLine,keymap} from '@codemirror/view';
import {foldGutter,indentOnInput,syntaxHighlighting,defaultHighlightStyle,bracketMatching,foldKeymap} from '@codemirror/language';
import {history,defaultKeymap,historyKeymap,indentMore,indentLess} from '@codemirror/commands';
import {highlightSelectionMatches,searchKeymap} from '@codemirror/search';
import {closeBrackets,closeBracketsKeymap} from '@codemirror/autocomplete';
import {lintKeymap} from '@codemirror/lint';

// Completion is registered exactly once by languageSupport. In particular this
// setup must not install basicSetup's hidden Enter-to-accept completion binding.
export const editingSetup=[
 lineNumbers(),highlightActiveLineGutter(),highlightSpecialChars(),history(),foldGutter(),drawSelection(),dropCursor(),
 EditorState.allowMultipleSelections.of(true),indentOnInput(),syntaxHighlighting(defaultHighlightStyle,{fallback:true}),
 bracketMatching(),closeBrackets(),rectangularSelection(),crosshairCursor(),highlightActiveLine(),highlightSelectionMatches(),
 keymap.of([...closeBracketsKeymap,...defaultKeymap,...searchKeymap,...historyKeymap,...foldKeymap,...lintKeymap,
  {key:'Tab',run:view=>!view.composing&&indentMore(view),shift:view=>!view.composing&&indentLess(view)}])
];
