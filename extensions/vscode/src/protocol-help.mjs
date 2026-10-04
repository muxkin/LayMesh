import {Lexer} from 'marked';

// LSP clients negotiate presentation independently for each language feature.
export function plainDocumentation(markdown) {
 const inline=tokens=>(tokens??[]).map(t=>{
  if(t.type==='link')return `${inline(t.tokens)} (${t.href})`;
  if(t.type==='image')return t.text;
  if(t.type==='html')return '';
  if(t.type==='br')return '\n';
  return t.tokens?inline(t.tokens):t.text??t.raw??'';
 }).join('');
 const blocks=tokens=>tokens.map(t=>{
  if(t.type==='space'||t.type==='html')return '';
  if(t.type==='list')return t.items.map((item,i)=>`${t.ordered?`${(Number(t.start)||1)+i}.`:'•'} ${blocks(item.tokens)}`).join('\n');
  if(t.type==='blockquote')return blocks(t.tokens);
  if(t.type==='table')return [t.header,...t.rows].map(row=>row.map(cell=>inline(cell.tokens)).join(' | ')).join('\n');
  if(t.type==='hr')return '—';
  return t.tokens?inline(t.tokens):t.text??'';
 }).filter(Boolean).join('\n\n');
 return blocks(Lexer.lex(markdown)).trim();
}
export function documentationFormat(capabilities,feature) {
 const options=capabilities.textDocument?.[feature];
 const formats=feature==='hover'?options?.contentFormat:feature==='signatureHelp'?options?.signatureInformation?.documentationFormat:options?.completionItem?.documentationFormat;
 const kind=formats?.find(kind=>kind==='markdown'||kind==='plaintext')??'plaintext';
 return markdown=>({kind,value:kind==='markdown'?markdown:plainDocumentation(markdown)});
}
