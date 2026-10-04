"""Native JSON-RPC regression tests; no Node extension packages are involved."""
import json,queue,subprocess,tempfile,threading,unittest
from pathlib import Path
from laymesh.bridge import _command
class NativeLanguageServer(unittest.TestCase):
 def setUp(self,initialization=None):
  self.process=subprocess.Popen([*_command(),'lsp','--stdio'],stdin=subprocess.PIPE,stdout=subprocess.PIPE,stderr=subprocess.PIPE);self.messages=queue.Queue();self.ident=0;self.configuration={};self.configuration_requests=0;self.configuration_registrations=0;self.published=[]
  def read():
   while True:
    header={}
    while True:
     line=self.process.stdout.readline()
     if not line:return
     if line in (b'\r\n',b'\n'):break
     key,value=line.decode().split(':',1);header[key.lower()]=value.strip()
    data=self.process.stdout.read(int(header['content-length']));self.messages.put(json.loads(data))
  threading.Thread(target=read,daemon=True).start()
  parameters={'locale':'en','capabilities':{'textDocument':{'completion':{'completionItem':{'documentationFormat':['markdown']}},'hover':{'contentFormat':['markdown']},'signatureHelp':{'signatureInformation':{'documentationFormat':['markdown'],'parameterInformation':{'labelOffsetSupport':True}}}}}}
  if initialization:parameters.update(initialization)
  result=self.request('initialize',parameters)
  self.assertEqual(result['capabilities']['positionEncoding'],'utf-16');self.notify('initialized',{})
 def send(self,message):
  body=json.dumps({'jsonrpc':'2.0',**message},ensure_ascii=False).encode();self.process.stdin.write(f'Content-Length: {len(body)}\r\n\r\n'.encode()+body);self.process.stdin.flush()
 def notify(self,method,params):self.send({'method':method,'params':params})
 def request(self,method,params,expected_error=False):
  self.ident+=1;ident=self.ident;self.send({'id':ident,'method':method,'params':params})
  while True:
   message=self.messages.get(timeout=15)
   if message.get('method')=='workspace/configuration':
    self.configuration_requests+=1;self.send({'id':message['id'],'result':[self.configuration]});continue
   if message.get('method')=='client/registerCapability':
    self.assertTrue(any(r['method']=='workspace/didChangeConfiguration' for r in message['params']['registrations']));self.configuration_registrations+=1;self.send({'id':message['id'],'result':None});continue
   if message.get('method')=='textDocument/publishDiagnostics':self.published.append(message['params'])
   if message.get('id')==ident:
    if expected_error:
     self.assertIn('error',message);return message['error']
    self.assertNotIn('error',message);return message.get('result')
 def tearDown(self):
  if self.process.poll() is None:
   self.request('shutdown',None);self.notify('exit',None)
   try:self.process.wait(timeout=5)
   except subprocess.TimeoutExpired:self.process.kill();self.process.wait()
  for stream in [self.process.stdin,self.process.stdout,self.process.stderr]:stream.close()
 def open(self,uri,text):self.notify('textDocument/didOpen',{'textDocument':{'uri':uri,'text':text,'version':1,'languageId':'laymesh'}})
 def test_workspace_references_rename_aliases_and_utf16_buffers(self):
  with tempfile.TemporaryDirectory(prefix='laymesh workspace 中文 ') as tmp:
   root=Path(tmp);library=root/'lib.lay';library.write_text('export width=10',encoding='utf-8')
   alias=root/'alias.lay';alias.write_text('import {width as w} from "./lib.lay"\nlabel=f"中文 😀 {w}"',encoding='utf-8')
   caller=root/'caller.lay';caller.write_text('import {width} from "./lib.lay"\nvalue=width',encoding='utf-8')
   ignored=root/'target';ignored.mkdir();(ignored/'ignored.lay').write_text('import {width} from "../lib.lay"\nx=width')
   self.notify('workspace/didChangeWorkspaceFolders',{'event':{'added':[{'uri':root.as_uri(),'name':'test'}],'removed':[]}})
   uri=library.as_uri();self.open(uri,library.read_text(encoding='utf-8'))
   params={'textDocument':{'uri':uri},'position':{'line':0,'character':8}}
   refs=self.request('textDocument/references',{**params,'context':{'includeDeclaration':True}})
   self.assertEqual(len(refs),6);self.assertEqual({r['uri'] for r in refs},{uri,alias.as_uri(),caller.as_uri()})
   usage=next(r for r in refs if r['uri']==alias.as_uri() and r['range']['start']['line']==1)
   self.assertEqual(usage['range'],{'start':{'line':1,'character':15},'end':{'line':1,'character':16}})
   highlight=self.request('textDocument/documentHighlight',params);self.assertEqual(highlight[0]['kind'],3)
   prepared=self.request('textDocument/prepareRename',params);self.assertEqual(prepared['placeholder'],'width')
   renamed=self.request('textDocument/rename',{**params,'newName':'extent'})
   self.assertEqual(len(renamed['changes'][alias.as_uri()]),1);self.assertEqual(len(renamed['changes'][caller.as_uri()]),2)
   self.open(alias.as_uri(),'import {width as w} from "./lib.lay"\nlabel=f"{w} {w}"')
   alias_params={'textDocument':{'uri':alias.as_uri()},'position':{'line':1,'character':9}}
   refs=self.request('textDocument/references',{**alias_params,'context':{'includeDeclaration':True}});self.assertEqual(len(refs),3)
   local=self.request('textDocument/rename',{**alias_params,'newName':'panel_width'});self.assertEqual(set(local['changes']),{alias.as_uri()})
   alias.write_text('import {width as w} from "./lib.lay"\nlabel=f"{w}"',encoding='utf-8')
   self.notify('workspace/didChangeWatchedFiles',{'changes':[{'uri':alias.as_uri(),'type':2}]})
   self.assertEqual(len(self.request('textDocument/references',{**alias_params,'context':{'includeDeclaration':True}})),3)
   self.notify('textDocument/didClose',{'textDocument':{'uri':alias.as_uri()}})
   self.assertEqual(len(self.request('textDocument/references',{**alias_params,'context':{'includeDeclaration':True}})),2)
   self.assertEqual(self.request('textDocument/rename',{**params,'newName':'if'},expected_error=True)['code'],-32602)
   self.assertEqual(self.request('textDocument/rename',{**params,'textDocument':{'uri':uri,'version':0},'newName':'extent'},expected_error=True)['code'],-32801)
   caller.unlink();self.notify('workspace/didChangeWatchedFiles',{'changes':[{'uri':caller.as_uri(),'type':3}]})
   self.assertEqual(len(self.request('textDocument/references',{**params,'context':{'includeDeclaration':True}})),4)
   self.notify('workspace/didChangeWorkspaceFolders',{'event':{'added':[],'removed':[{'uri':root.as_uri(),'name':'test'}]}})
   self.assertEqual(len(self.request('textDocument/references',{**params,'context':{'includeDeclaration':True}})),1)
 def test_multi_root_initialization_and_versioned_rename_edits(self):
  with tempfile.TemporaryDirectory() as first,tempfile.TemporaryDirectory() as second:
   root1=Path(first);root2=Path(second);library=root1/'lib.lay';library.write_text('export amount=10')
   consumer=root2/'caller.lay';consumer.write_text(f'import {{amount}} from "{library.as_posix()}"\nresult=amount')
   # Start a fresh server with both roots in its initial handshake.
   self.tearDown();self.setUp({'locale':'en','workspaceFolders':[{'uri':root1.as_uri(),'name':'one'},{'uri':root2.as_uri(),'name':'two'}],'capabilities':{'workspace':{'workspaceFolders':True,'workspaceEdit':{'documentChanges':True}}}})
   self.open(library.as_uri(),library.read_text());params={'textDocument':{'uri':library.as_uri()},'position':{'line':0,'character':8}}
   refs=self.request('textDocument/references',{**params,'context':{'includeDeclaration':False}});self.assertEqual(len(refs),2)
   edit=self.request('textDocument/rename',{**params,'newName':'quantity'});changes={c['textDocument']['uri']:c for c in edit['documentChanges']}
   self.assertEqual(changes[library.as_uri()]['textDocument']['version'],1);self.assertIsNone(changes[consumer.as_uri()]['textDocument']['version'])
 def test_completion_hover_and_named_signature(self):
  uri='file:///test.lay';source='page=canvas(size=(100,80))\np=plot(size=(80,60))\np.line(x=[0,1],y=[1,2], line_';self.open(uri,source)
  result=self.request('textDocument/completion',{'textDocument':{'uri':uri},'position':{'line':2,'character':len(source.splitlines()[2])}})
  self.assertIn('line_width',[c['label'] for c in result])
  sig=self.request('textDocument/signatureHelp',{'textDocument':{'uri':uri},'position':{'line':2,'character':len(source.splitlines()[2])}});self.assertTrue(sig['signatures'][0]['label'].startswith('plot.line('))
  hover=self.request('textDocument/hover',{'textDocument':{'uri':uri},'position':{'line':0,'character':7}});self.assertIn('Create a page',hover['contents']['value'])
 def test_disk_imports_transitive_documentation_and_definition(self):
  with tempfile.TemporaryDirectory() as tmp:
   root=Path(tmp);(root/'card.lay').write_text('## A reusable component.\n## @param name - Component name.\nexport function card(name,size=(40,25)) {return name}\n');(root/'wrapper.lay').write_text('import {card} from "./card.lay"\nexport alias=card\n');uri=(root/'main.lay').as_uri();source='import {alias as tile} from "./wrapper.lay"\ntile(';self.open(uri,source)
   sig=self.request('textDocument/signatureHelp',{'textDocument':{'uri':uri},'position':{'line':1,'character':5}});self.assertIn('A reusable component',sig['signatures'][0]['documentation']['value'])
   definition=self.request('textDocument/definition',{'textDocument':{'uri':uri},'position':{'line':1,'character':2}});self.assertEqual(definition['uri'],(root/'card.lay').as_uri())
 def test_incremental_utf16_change(self):
  uri='file:///unicode.lay';self.open(uri,'# 😀 中文\npage=canvas(size=(40,30))\npage.add(')
  self.notify('textDocument/didChange',{'textDocument':{'uri':uri,'version':2},'contentChanges':[{'range':{'start':{'line':0,'character':2},'end':{'line':0,'character':4}},'text':'𐐀'}]})
  sig=self.request('textDocument/signatureHelp',{'textDocument':{'uri':uri},'position':{'line':2,'character':9}});self.assertTrue(sig['signatures'][0]['label'].startswith('add('))
 def test_percent_encoded_imports_read_disk(self):
  with tempfile.TemporaryDirectory(prefix='laymesh URI 中文 ') as tmp:
   root=Path(tmp);module=root/'模块 #%.lay';module.write_text('## Encoded module documentation.\nexport function card(title){return title}\n',encoding='utf-8')
   for escaped in [False,True]:
    uri=(root/f'main{escaped}.lay').as_uri();source=json.dumps('./'+module.name,ensure_ascii=escaped);self.open(uri,'import {card} from '+source+'\ncard(')
    signature=self.request('textDocument/signatureHelp',{'textDocument':{'uri':uri},'position':{'line':1,'character':5}});self.assertIn('Encoded module documentation',signature['signatures'][0]['documentation']['value'])
   definition=self.request('textDocument/definition',{'textDocument':{'uri':uri},'position':{'line':1,'character':2}});self.assertEqual(definition['uri'],module.as_uri())
 def test_located_diagnostic_quick_fix(self):
  uri='file:///fix.lay';source='page=canvas(size=(40,30))\nr=rect(size=(3,2),stroke_width=1pt)';self.open(uri,source)
  while True:
   message=self.messages.get(timeout=15)
   if message.get('method')=='textDocument/publishDiagnostics':break
  issue=next(d for d in message['params']['diagnostics'] if d['code']=='E_API_MIGRATION');self.assertEqual(issue['data']['replacement'],'border_width')
  actions=self.request('textDocument/codeAction',{'textDocument':{'uri':uri},'range':issue['range'],'context':{'diagnostics':[issue]}});self.assertEqual(actions[0]['edit']['changes'][uri][0]['newText'],'border_width')

 def test_pull_configuration_markdown_negotiation_and_locale_switch(self):
  self.configuration={'language':'en'}
  capabilities={'workspace':{'configuration':True,'didChangeConfiguration':{'dynamicRegistration':True}},'textDocument':{'completion':{'completionItem':{'documentationFormat':['plaintext','markdown']}},'hover':{'contentFormat':['markdown']},'signatureHelp':{'signatureInformation':{'documentationFormat':['markdown'],'parameterInformation':{'labelOffsetSupport':True}}}}}
  self.request('initialize',{'locale':'zh-CN','initializationOptions':{'language':'zh-CN'},'capabilities':capabilities});self.notify('initialized',{})
  module='file:///localization-lib.lay';uri='file:///localization-main.lay';library='## @lang zh-CN\n## 创建面板。\n## @param title - 面板标题。\n## @lang en\n## Create a **panel**.\n## @param title - Panel title.\nexport function card(title) {return group()}'
  self.open(module,library);self.open(uri,'import {card as tile} from "./localization-lib.lay"\ntile(')
  params={'textDocument':{'uri':uri},'position':{'line':1,'character':5}}
  signature=self.request('textDocument/signatureHelp',params);self.assertIn('**panel**',signature['signatures'][0]['documentation']['value']);self.assertIsInstance(signature['signatures'][0]['parameters'][0]['label'],list)
  completion=self.request('textDocument/completion',params);self.assertEqual(next(c for c in completion if c['label']=='title')['documentation']['kind'],'plaintext')
  self.configuration={'language':'auto'};self.notify('workspace/didChangeConfiguration',{'settings':{'laymesh':self.configuration}})
  signature=self.request('textDocument/signatureHelp',params);self.assertIn('创建面板',signature['signatures'][0]['documentation']['value']);self.assertEqual(self.configuration_registrations,1);self.assertGreaterEqual(self.configuration_requests,2)

 def legacy_documents(self,initialize=None):
  self.request('initialize',{'capabilities':{},**(initialize or {})});self.notify('initialized',{})
  self.lib_uri='file:///tmp/laymesh-language-library.lay';self.main_uri='file:///tmp/laymesh-language-main.lay'
  self.library='## @lang zh-CN\n## 创建面板。\n## @param {string} title - 面板标题。\n## @returns {group} 组合。\n## @lang en\n## Create a **panel**.\n## @param {string} title - Panel title.\n## @returns {group} A group.\nexport function card(title) {return group()}'
  self.caller='import {card as tile} from "./laymesh-language-library.lay"\nvalue=tile(title="Result")'
  self.open(self.lib_uri,self.library);self.open(self.main_uri,self.caller)
 def legacy_query(self,method,character):return self.request('textDocument/'+method,{'textDocument':{'uri':self.main_uri},'position':{'line':1,'character':character}})
 def test_legacy_plaintext_defaults_and_parameter_strings(self):
  self.legacy_documents();s=self.legacy_query('signatureHelp',12)
  self.assertEqual(s['signatures'][0]['documentation']['kind'],'plaintext');self.assertIn('Create a panel',s['signatures'][0]['documentation']['value']);self.assertNotIn('**',s['signatures'][0]['documentation']['value']);self.assertEqual(s['signatures'][0]['parameters'][0]['label'],'title')
  self.assertIn('panel',self.legacy_query('hover',7)['contents']['value']);self.assertIn('Panel title',next(c for c in self.legacy_query('completion',11) if c['label']=='title')['documentation']['value']);self.assertEqual(self.configuration_requests,0)
 def test_legacy_push_configuration_preserves_documents(self):
  self.legacy_documents({'locale':'en','initializationOptions':{'language':'zh-CN'}})
  self.assertIn('创建面板',self.legacy_query('signatureHelp',12)['signatures'][0]['documentation']['value'])
  for config,method,expected in [({'language':'en'},'hover','Create a panel'),({'language':'auto'},'signatureHelp','Create a panel'),({},'signatureHelp','创建面板')]:
   self.notify('workspace/didChangeConfiguration',{'settings':{'laymesh':config}});result=self.legacy_query(method,7 if method=='hover' else 12);self.assertIn(expected,result['contents']['value'] if method=='hover' else result['signatures'][0]['documentation']['value'])
  self.notify('textDocument/didChange',{'textDocument':{'uri':self.lib_uri,'version':2},'contentChanges':[{'text':self.library.replace('面板标题。','更新后的标题。')}]});self.assertIn('更新后的标题',self.legacy_query('signatureHelp',12)['signatures'][0]['parameters'][0]['documentation']['value'])
 def test_legacy_pull_configuration_formats_and_diagnostic_locale(self):
  self.configuration={'language':'en'}
  caps={'workspace':{'configuration':True,'didChangeConfiguration':{'dynamicRegistration':True}},'textDocument':{'completion':{'completionItem':{'documentationFormat':['plaintext','markdown']}},'hover':{'contentFormat':['markdown']},'signatureHelp':{'signatureInformation':{'documentationFormat':['markdown'],'parameterInformation':{'labelOffsetSupport':True}}}}}
  self.legacy_documents({'locale':'zh-Hans','initializationOptions':{'language':'zh-CN'},'capabilities':caps});s=self.legacy_query('signatureHelp',12)
  self.assertIn('**panel**',s['signatures'][0]['documentation']['value']);self.assertIsInstance(s['signatures'][0]['parameters'][0]['label'],list);self.assertEqual(next(c for c in self.legacy_query('completion',11) if c['label']=='title')['documentation']['kind'],'plaintext');self.assertEqual(self.legacy_query('hover',7)['contents']['kind'],'markdown')
  self.configuration={'language':'auto'};self.notify('workspace/didChangeConfiguration',{'settings':{'laymesh':self.configuration}});self.assertIn('创建面板',self.legacy_query('signatureHelp',12)['signatures'][0]['documentation']['value']);self.assertGreaterEqual(self.configuration_requests,2);self.assertEqual(self.configuration_registrations,1)
  self.notify('textDocument/didChange',{'textDocument':{'uri':self.lib_uri,'version':2},'contentChanges':[{'text':self.library.replace('@param {string} title','@param {string} typo')}]});self.legacy_query('signatureHelp',12)
  self.assertTrue(any(w['code']=='W_DOC' and '不在函数' in w['message'] for d in self.published for w in d['diagnostics']))
  self.configuration={'language':'en'};self.notify('workspace/didChangeConfiguration',{'settings':{'laymesh':self.configuration}});self.legacy_query('signatureHelp',12);self.assertTrue(any(w['code']=='W_DOC' and 'not in the declaration' in w['message'] for d in self.published for w in d['diagnostics']))
 def test_legacy_packaged_protocol_and_imported_doc_updates(self):
  uri='file:///tmp/laymesh-lsp-check.lay';source='page=canvas(size=(100,80))\na=rect(size=(40,25),stroke_width=1pt)';self.open(uri,source)
  def query(method,char):return self.request('textDocument/'+method,{'textDocument':{'uri':uri},'position':{'line':1,'character':char}})
  self.assertIn('rect',query('hover',3)['contents']['value']);self.assertTrue(any(c['label']=='size' for c in query('completion',7)));self.assertTrue(query('signatureHelp',8)['signatures'][0]['label'].startswith('rect('))
  ds=next(d['diagnostics'] for d in self.published if d['uri']==uri);self.assertTrue(any(d['code']=='E_API_MIGRATION' for d in ds));actions=self.request('textDocument/codeAction',{'textDocument':{'uri':uri},'range':{'start':{'line':1,'character':20},'end':{'line':1,'character':32}},'context':{'diagnostics':ds}});self.assertTrue(any(a['edit']['changes'][uri][0]['newText']=='border_width' for a in actions))
  module='file:///tmp/laymesh-lsp-library.lay';library='## Build a library panel.\n## @param {string} title - Panel title.\n## @param {size} size - Physical dimensions.\n## @returns {group} Reusable panel.\nexport function card(title, size=(40,25)) {return group()}';self.open(module,library);caller='import {card as tile} from "./laymesh-lsp-library.lay"\nvalue=tile("x", size=';self.notify('textDocument/didChange',{'textDocument':{'uri':uri,'version':2},'contentChanges':[{'text':caller}]});s=query('signatureHelp',len(caller.splitlines()[1]));self.assertIn('Build a library panel',s['signatures'][0]['documentation']['value']);self.assertIn('Physical dimensions',s['signatures'][0]['parameters'][1]['documentation']['value']);self.assertEqual(s['activeParameter'],1);label=s['signatures'][0]['label'];a,b=s['signatures'][0]['parameters'][1]['label'];self.assertEqual(label[a:b],'size=(40,25)')
  self.notify('textDocument/didChange',{'textDocument':{'uri':module,'version':2},'contentChanges':[{'text':library.replace('40,25','50,30').replace('Physical dimensions','Updated physical dimensions')}]});s=query('signatureHelp',len(caller.splitlines()[1]));self.assertIn('Updated physical dimensions',s['signatures'][0]['parameters'][1]['documentation']['value']);self.assertIn('50,30',s['signatures'][0]['label'])
 def test_closed_module_disk_refresh_and_open_buffer_priority(self):
  with tempfile.TemporaryDirectory() as tmp:
   root=Path(tmp);module=root/'card.lay';module.write_text('## Disk version.\nexport function card(disk=1) {return disk}');uri=(root/'main.lay').as_uri();source='import {card} from "./card.lay"\ncard(';self.open(uri,source)
   def signature():return self.request('textDocument/signatureHelp',{'textDocument':{'uri':uri},'position':{'line':1,'character':5}})
   self.assertIn('disk=1',signature()['signatures'][0]['label'])
   self.open(module.as_uri(),'## Unsaved buffer.\nexport function card(buffer=2) {return buffer}');self.assertIn('buffer=2',signature()['signatures'][0]['label'])
   module.write_text('## Changed on disk.\nexport function card(changed=3) {return changed}')
   self.notify('workspace/didChangeWatchedFiles',{'changes':[{'uri':module.as_uri(),'type':2}]});self.assertIn('buffer=2',signature()['signatures'][0]['label'])
   self.notify('textDocument/didClose',{'textDocument':{'uri':module.as_uri()}});self.assertIn('changed=3',signature()['signatures'][0]['label'])
   module.write_text('export function card(external=4) {return external}');self.assertIn('external=4',signature()['signatures'][0]['label'])
   published_before_delete=len(self.published)
   module.unlink();self.notify('workspace/didChangeWatchedFiles',{'changes':[{'uri':module.as_uri(),'type':3}]});self.assertIsNone(signature())
   self.assertTrue(any(d['uri']==uri and d['version']==1 for d in self.published[published_before_delete:]))
 def test_lcss_and_utf16_typed_diagnostics(self):
  uri='file:///audit-types.lay';source='# 😀 中文\nstyle { text { nonsense:1pt; } }\npage=canvas(size=(10deg,20))\nt=text(font_family=["sans",3])\nr=rect(border_width=true)';self.open(uri,source);self.request('textDocument/hover',{'textDocument':{'uri':uri},'position':{'line':1,'character':16}})
  ds=next(d['diagnostics'] for d in reversed(self.published) if d['uri']==uri);self.assertEqual([d['code'] for d in ds],['E_LCSS','E_UNIT','E_TYPE','E_TYPE']);self.assertEqual(ds[0]['range']['start'],{'line':1,'character':15})
  for d in ds:self.assertGreaterEqual(d['range']['end']['line'],d['range']['start']['line'])
 def test_unicode_syntax_error_keeps_server_alive(self):
  uri='file:///unicode-invalid.lay';self.open(uri,'# 😀\n😀')
  self.request('textDocument/hover',{'textDocument':{'uri':uri},'position':{'line':1,'character':0}})
  diagnostics=next(d['diagnostics'] for d in reversed(self.published) if d['uri']==uri);self.assertEqual(diagnostics[0]['range'],{'start':{'line':1,'character':0},'end':{'line':1,'character':2}})
  self.notify('textDocument/didChange',{'textDocument':{'uri':uri,'version':2},'contentChanges':[{'text':'page=canvas(size=(20,10))\npage.add('}]});signature=self.request('textDocument/signatureHelp',{'textDocument':{'uri':uri},'position':{'line':1,'character':9}});self.assertTrue(signature['signatures'][0]['label'].startswith('add('))
