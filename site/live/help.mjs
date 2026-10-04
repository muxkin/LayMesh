import {marked} from 'marked';
import {EditorView} from '@codemirror/view';

const en=document.documentElement.lang==='en';
const element=(tag,text)=>{const el=document.createElement(tag);if(text!==undefined)el.textContent=text;return el;};
function safeLink(href){
 try {
  if(/^https:\/\/muxkin\.github\.io\/LayMesh\//.test(href))return new URL(href.split('/LayMesh/')[1],new URL('../../',import.meta.url)).href;
  const url=new URL(href,location.href);return ['http:','https:','mailto:'].includes(url.protocol)?url.href:undefined;
 }catch{return undefined;}
}
// Build a small Markdown subset as DOM nodes. Never insert HTML or fetch images.
function render(tokens,parent){
 for(const token of tokens??[]){
  let node;
  switch(token.type){
   case 'space':continue;
   case 'paragraph':case 'heading':node=element(token.type==='heading'?'strong':'p');render(token.tokens,node);break;
   case 'text':node=document.createDocumentFragment();if(token.tokens)render(token.tokens,node);else node.append(document.createTextNode(token.text));break;
   case 'strong':case 'em':case 'del':node=element(token.type==='del'?'s':token.type);render(token.tokens,node);break;
   case 'codespan':node=element('code',token.text);break;
   case 'code':node=element('pre');node.append(element('code',token.text));break;
   case 'br':node=element('br');break;
   case 'hr':node=element('hr');break;
   case 'list':node=element(token.ordered?'ol':'ul');if(token.ordered&&token.start!==1)node.start=token.start;for(const item of token.items){const li=element('li');render(item.tokens,li);node.append(li);}break;
   case 'blockquote':node=element('blockquote');render(token.tokens,node);break;
   case 'link':{const href=safeLink(token.href);node=element(href?'a':'span');if(href){node.href=href;node.rel='noopener noreferrer';}render(token.tokens,node);break;}
   case 'image':node=element('span',token.text);break;
   case 'escape':node=document.createTextNode(token.text);break;
   default:node=document.createTextNode(token.text??token.raw??'');
  }
  parent.append(node);
 }
}
export function helpDOM(markdown){const dom=element('div');dom.className='language-help';render(marked.lexer(markdown??''),dom);return dom;}
export function signatureDOM(result,view){
 const dom=element('section');dom.className='language-help signature-help';dom.setAttribute('aria-label',en?'Call documentation':'方法与参数说明');
 const method=helpDOM(result.summary||result.documentation.split(/\n\s*\n/)[0]);method.classList.add('signature-purpose');dom.append(method);
 if(result.returns){const returns=helpDOM(`**${en?'Returns':'返回'}**: ${result.returns}`);returns.classList.add('signature-returns');dom.append(returns);}
 const details=element('details');details.className='signature-details';
 const summary=element('summary'),code=element('code'),parameter=result.parameters[result.activeParameter];
 const drawSignature=()=>{
  code.replaceChildren();
  const add=(text,active=false)=>code.append(active?element('mark',text):document.createTextNode(text));
  if(details.open||result.parameters.length<=4){
   if(parameter){const [from,to]=parameter.label;add(result.label.slice(0,from));add(result.label.slice(from,to),true);add(result.label.slice(to));}
   else add(result.label);
  }else {
   add(result.name+'(');const selected=result.activeParameter??1,indices=[...new Set([0,selected])].sort((a,b)=>a-b);let last=-1;
   for(const index of indices){const p=result.parameters[index];if(!p)continue;if(index>last+1)add(last>=0?', …, ':'…, ');else if(last>=0)add(', ');
    add(result.label.slice(...p.label),index===result.activeParameter);last=index;
   }
   if(last<result.parameters.length-1)add(', …');add(')');
  }
 };
 drawSignature();
 const hint=element('span',en?'Expand signature':'展开签名');hint.className='signature-expand';summary.append(code,hint);details.append(summary);
 details.addEventListener('toggle',()=>{drawSignature();hint.textContent=details.open?(en?'Collapse signature':'收起签名'):(en?'Expand signature':'展开签名');view.requestMeasure();});dom.append(details);
 if(parameter){
  const label=element('div',en?'CURRENT PARAMETER':'当前参数');label.className='signature-label';dom.append(label);
  dom.append(helpDOM(`**${parameter.name}** — \`${parameter.type}\`\n\n${parameter.description}`));
  const meta=[parameter.unit?`${en?'Unit':'单位'}: ${parameter.unit}`:'',parameter.default!==undefined?`${en?'Default':'默认'}: \`${parameter.default}\``:parameter.required?(en?'Required':'必需'):''].filter(Boolean).join(' · ');
  if(meta)dom.append(helpDOM(meta));
  if(parameter.example)dom.append(helpDOM('```lay\n'+parameter.example+'\n```'));
 }
 return dom;
}
export function positionInfo(view,list,option,info,space){
 const small=space.right-space.left<700,availableRight=space.right-list.right-8,availableLeft=list.left-space.left-8;
 if(small||Math.max(availableLeft,availableRight)<240){
  // In normal flow the list and documentation are measured as one tooltip.
  return {style:'position:static; width:auto; max-width:100%; max-height:30vh;',class:'language-info-below'};
 }
 const right=availableRight>=Math.min(380,availableLeft),width=Math.min(400,right?availableRight:availableLeft);
 const top=Math.max(space.top+8,Math.min(option.top,space.bottom-Math.min(info.height,320)-8))-list.top;
 return {style:`position:absolute; ${right?'left':'right'}:100%; top:${top}px; width:${width}px; max-width:${width}px; max-height:${Math.min(320,space.bottom-space.top-16)}px;`,class:'language-info-side'};
}
export const helpTheme=EditorView.baseTheme({
 '.language-help':{boxSizing:'border-box',font:'13px/1.55 system-ui, sans-serif',whiteSpace:'normal',padding:'12px',maxWidth:'min(440px, calc(100vw - 24px))',maxHeight:'min(360px, 42vh)',overflow:'auto',overflowWrap:'anywhere'},
 '.language-help p':{margin:'0 0 8px'},'.language-help p:last-child':{marginBottom:'0'},
 '.language-help strong':{fontWeight:'650'},
 '.language-help code':{font:'12px/1.5 var(--mono, monospace)',color:'#cceadb',background:'#ffffff0b',border:'0',borderRadius:'3px',padding:'1px 3px'},
 '.language-help pre':{padding:'9px',margin:'8px 0',border:'1px solid #ffffff20',borderRadius:'5px',whiteSpace:'pre-wrap',overflowWrap:'anywhere',background:'#182c25'},
 '.language-help pre code':{padding:'0',background:'none',whiteSpace:'pre-wrap'},
 '.language-help a':{color:'#8ee1c6',textDecoration:'underline'},
 '.language-help ul,.language-help ol':{paddingLeft:'20px',margin:'8px 0'},
 '.language-help li':{margin:'3px 0'},'.language-help blockquote':{margin:'8px 0',paddingLeft:'10px',borderLeft:'2px solid #669b86'},
 '.signature-help > .language-help':{padding:'0',maxHeight:'none',overflow:'visible'},
 '.signature-help .signature-purpose':{fontSize:'13px'},
 '.signature-returns':{margin:'7px 0',color:'#b4cec3'},
 '.signature-details':{margin:'10px 0',padding:'9px',border:'1px solid #ffffff25',borderRadius:'5px',background:'#152d24'},
 '.signature-details summary':{cursor:'pointer',display:'block',listStyle:'none'},
 '.signature-details summary code':{background:'none',padding:'0',display:'-webkit-box',WebkitBoxOrient:'vertical',WebkitLineClamp:'2',overflow:'hidden',whiteSpace:'pre-wrap'},
 '.signature-details[open] summary code':{display:'block'},
 '.signature-details mark':{color:'#132b21',background:'#94e3c6',borderRadius:'2px'},
 '.signature-expand':{display:'block',fontSize:'11px',color:'#93d9bd',marginTop:'5px'},
 '.signature-label':{fontSize:'11px',letterSpacing:'.06em',color:'#acc9bc',margin:'10px 0 6px'},
 '.cm-tooltip-autocomplete':{maxWidth:'min(340px, calc(100vw - 24px))'},
 '.cm-tooltip-autocomplete > ul':{maxHeight:'min(210px, 25vh)'},
 '.cm-tooltip.cm-completionInfo':{padding:'0',overflow:'auto',boxSizing:'border-box',borderRadius:'5px'},
 '.cm-completionInfo .language-help':{maxHeight:'none',maxWidth:'none'},
 '.cm-completionInfo.language-info-below':{borderTop:'1px solid #668474',borderRadius:'0 0 5px 5px'},
 '.cm-tooltip':{borderRadius:'6px',boxShadow:'0 8px 24px #0004'},
 '@media (max-width:700px)':{'.cm-tooltip-autocomplete':{width:'min(320px, calc(100vw - 24px))'},'.cm-tooltip-autocomplete > ul':{maxHeight:'20vh'},'.signature-help':{maxHeight:'32vh'}}
});
