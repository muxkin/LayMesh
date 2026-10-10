import {languageSupport} from './language.mjs';
import { EditorView } from '@codemirror/view';
import { editingSetup } from './setup.mjs';
import { EditorState, Compartment, ChangeSet } from '@codemirror/state';
import { keymap } from '@codemirror/view';
import { tags } from '@lezer/highlight';
import { HighlightStyle, syntaxHighlighting } from '@codemirror/language';
import { json } from '@codemirror/lang-json';
const theme=EditorView.theme({
 '&':{fontSize:'14px',height:'100%',lineHeight:'1.8',backgroundColor:'var(--code)' ,color:'#dcece7'},
 '.cm-content':{fontFamily:'var(--mono, monospace)',padding:'14px 0',caretColor:'#ffffff'},
 '.cm-gutters':{backgroundColor:'var(--code)',color:'#91aaa2',border:'none'},
 '.cm-activeLine,.cm-activeLineGutter':{backgroundColor:'#ffffff08'},
 '&.cm-focused .cm-selectionBackground,.cm-selectionBackground':{backgroundColor:'#346355 !important'},
 '.cm-scroller':{overflow:'auto',minHeight:'0',height:'100%'},
 '.cm-cursor':{borderLeftColor:'#fff'},
 '.cm-tooltip':{backgroundColor:'#253d35',color:'#e7f3ef',border:'1px solid #536b63'},
},{dark:true});
const colors=HighlightStyle.define([{tag:tags.keyword,color:'#7ccde0'},{tag:tags.function(tags.variableName),color:'#91d6c6'},{tag:tags.propertyName,color:'#a7cdea'},{tag:tags.operator,color:'#aabfba'},{tag:tags.string,color:'#b9dd91'},{tag:tags.number,color:'#e5c48c'},{tag:tags.comment,color:'#8caaa0'}]);
function extensions(file,wrap){return [editingSetup,theme,syntaxHighlighting(colors),file.endsWith('.json')?json():/\.(lay|laydoc|lcss|css)$/.test(file)?languageSupport(this,file):[],wrap.of(EditorView.lineWrapping),keymap.of([{key:'Mod-Enter',run:()=>{this.schedule(true);return true;}}]),EditorView.domEventHandlers({compositionstart:()=>{this.composing=true;this.seq++;this.pending=false;clearTimeout(this.debounce);},compositionend:()=>{this.composing=false;this.schedule();}}),EditorView.updateListener.of(update=>{if(!update.docChanged)return;this.maps.set(file,this.maps.get(file).composeDesc(update.changes.desc));this.files[file]=update.state.doc.toString();if(!this.composing)this.schedule();else this.changed();})];}
export function editor(panel){const file=panel.dataset.editFile;if(this.views.has(file))return;this.maps.set(file,ChangeSet.empty(this.files[file].length).desc);
  const code=panel.querySelector('.code-block');code.getSource=()=>this.files[file];code.querySelector('pre').hidden=true;const host=document.createElement('div');host.className='live-editor';code.append(host);const wrap=new Compartment();
  const view=new EditorView({state:EditorState.create({doc:this.files[file],extensions:extensions.call(this,file,wrap)}),parent:host});view.contentDOM.setAttribute('aria-label',file);view.contentDOM.setAttribute('spellcheck','false');
  code.querySelector('.wrap-code').addEventListener('click',()=>view.dispatch({effects:wrap.reconfigure(code.classList.contains('wrap')?EditorView.lineWrapping:[])}));this.views.set(file,{view,wrap});
 }
export function reset(){
 this.maps.clear();
 for(const [file,{view,wrap}]of this.views){this.maps.set(file,ChangeSet.empty(this.files[file].length).desc);view.setState(EditorState.create({doc:this.files[file],extensions:extensions.call(this,file,wrap)}));this.module.querySelector(`[data-edit-file="${CSS.escape(file)}"] .code-raw`).value=this.files[file];}
}
export function focusEntry(offset){
 const view=this.views.get(this.entry)?.view;if(!view)return;
 const anchor=this.maps.get(this.entry).mapPos(offset,1);
 view.dispatch({selection:{anchor},effects:EditorView.scrollIntoView(anchor,{y:'center'})});view.focus();
}
