#!/usr/bin/env python3
"""Exercise negotiated source deltas, drawing patches and transactional recovery."""
import argparse, copy, json, select, subprocess, tempfile, time
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1]
def apply(old,update):
    result=copy.deepcopy(old)
    def node(path):
        nodes=result['nodes']
        for i,index in enumerate(path):
            n=nodes[index]
            if i+1<len(path):nodes=n['kind']['children']
        return n
    for key in ('animation','pageTransition','width','height','schemaVersion'):
        if update.get(key) is None:result.pop(key,None)
        else:result[key]=update[key]
    if update.get('roots') is not None:result['nodes']=update['roots']
    for p in update.get('properties',[]):
        n=node(p['path'])
        for key,value in p['values'].items():
            if key.startswith('kind.'):n['kind'][key[5:]]=value
            else:n[key]=value
    for p in update['nodes']:
        nodes=node(p['path'][:-1])['kind']['children'] if len(p['path'])>1 else result['nodes']
        nodes[p['path'][-1]]=p['node']
    for s in sorted(update.get('structure',[]),key=lambda s:-len(s['parent'])):
        nodes=node(s['parent'])['kind']['children'] if s['parent'] else result['nodes']
        inserted=[nodes[x['existing']] if 'existing' in x else x['new'] for x in s['insert']]
        nodes[s['start']:s['start']+s['delete']]=inserted
    if update.get('resources') is not None:result['resources']=update['resources']
    return result

def canonical(value):
    if isinstance(value,dict):return {k:canonical(v) for k,v in value.items() if k!='_identity'}
    if isinstance(value,list):return [canonical(v) for v in value]
    return value

def main():
    parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('--binary',type=Path,default=ROOT/'target/release/laymesh');parser.add_argument('--output',type=Path)
    args=parser.parse_args();rows=[];scope_rows=[]
    with tempfile.TemporaryDirectory(prefix='laymesh-session-') as tmp:
        work=Path(tmp);file=str(work/'main.lay')
        proc=subprocess.Popen([str(args.binary),'preview','--stdio'],stdin=subprocess.PIPE,stdout=subprocess.PIPE,text=True,bufsize=1)
        def receive():
            if not select.select([proc.stdout],[],[],60)[0]:raise TimeoutError('Native session did not respond')
            line=proc.stdout.readline();assert line,'Native preview stopped';return json.loads(line)
        def send(request):proc.stdin.write(json.dumps(request)+'\n');proc.stdin.flush()
        ready=receive();assert 'incremental-session-v1' in ready['capabilities']
        base=None;drawing=None
        source='p=canvas(size=(80,50))\nfunction item(i) {g=group()\ng.add(path(commands=[move_to(0,0),cubic_to(2,i,3,4,5,6)],fill="#336699"))\nreturn g}\nfor i in range(8) {p.add(item(i),offset=(i*8,0))}'
        try:
            for version,text in enumerate([source,source,source.replace('range(8)','range(9)'),source.replace('#336699','#339966'),source+'\ninvalid=(',source],1):
                request=dict(id=version,file=file,protocol=3,resource_dir=tmp,incremental_session='incremental-session-v1',sourceVersion=version,changes={file:text},display_list_updates=True,known_display_list=base)
                start=time.perf_counter();send(request);response=receive();elapsed=(time.perf_counter()-start)*1000
                assert response['sourceVersion']==version and response['appliedSourceVersion']==version
                if version==5:
                    assert 'error' in response;continue
                assert 'error' not in response,response
                if 'displayListUpdate' in response:
                    assert response['displayListUpdate']['base']==base
                    drawing=apply(drawing,response['displayListUpdate']['update'])
                else:drawing=response['displayList']
                base=response['displayListId']
                # Legacy requests exercise the uncached oracle and old protocol.
                send(dict(id=100+version,file=file,source=text,protocol=3,resource_dir=tmp));oracle=receive()
                assert 'error' not in oracle,oracle
                assert canonical(drawing)==canonical(oracle['displayList']),f'Drawing differs from uncached oracle at revision {version}'
                rows.append(dict(version=version,elapsedMs=elapsed,responseBytes=len(json.dumps(response)),metrics=response['compileMetrics']))
            assert rows[1]['metrics']['evaluatedBuiltinCalls'].get('path',0)==0
            assert rows[2]['metrics']['evaluatedBuiltinCalls'].get('path')==1
            scope_source='''p=canvas(size=(400,100))
function curve(i) {g=group() g.add(path(commands=[move_to(0,0),cubic_to(2,i+1,3,4,5,6)])) return g}
function other(i) {g=group() g.add(ellipse(size=(i+1,2))) return g}
for i in range(128) {p.add(curve(i),offset=(i*2,0))}
for i in range(32) {p.add(other(i),offset=(i*2,30))}
'''
            formula=scope_source.replace('i+1,3,4','i+1001,3,4')
            for version,(label,text) in enumerate([('initial',scope_source),('warm',scope_source),('formula',formula),('loop_growth',formula.replace('range(128)','range(129)')),('undo',scope_source)],7):
                start=time.perf_counter()
                send(dict(id=version,file=file,protocol=3,resource_dir=tmp,incremental_session='incremental-session-v1',sourceVersion=version,changes={file:text},display_list_updates=True,known_display_list=base))
                response=receive();elapsed=(time.perf_counter()-start)*1000
                assert 'error' not in response,response
                drawing=apply(drawing,response['displayListUpdate']['update']) if 'displayListUpdate' in response else response['displayList']
                base=response['displayListId']
                send(dict(id=100+version,file=file,source=text,protocol=3,resource_dir=tmp));oracle=receive()
                assert 'error' not in oracle,oracle
                assert canonical(drawing)==canonical(oracle['displayList']),label
                metrics=response['compileMetrics'];calls=metrics['evaluatedBuiltinCalls']
                if label=='warm':assert calls.get('path',0)==0
                if label=='formula':assert calls.get('path')==128 and calls.get('ellipse',0)==0
                if label=='loop_growth':assert calls.get('path')==1 and calls.get('ellipse',0)==0
                scope_rows.append(dict(change=label,elapsedMs=elapsed,metrics=metrics,responseBytes=len(json.dumps(response))))
            long='p=canvas(size=(80,50))\nfor i in range(10000) {for j in range(10000) {x=(i+j)*(i-j)}}'
            send(dict(id=20,file=file,protocol=3,resource_dir=tmp,incremental_session='incremental-session-v1',sourceVersion=20,changes={file:long}))
            time.sleep(.03);send(dict(type='cancel',id=20))
            response=receive();assert response['id']==20 and response['error']['code']=='E_CANCELLED',response
            send(dict(id=21,file=file,protocol=3,resource_dir=tmp,incremental_session='incremental-session-v1',sourceVersion=21,changes={file:source}))
            assert 'error' not in receive()
        finally:
            proc.stdin.close();proc.wait(timeout=15)
    result={'passed':True,'cases':rows,'scopeCases':scope_rows,'scopeTimingDefinition':'One native request through response, excluding the separate uncached oracle; single samples, not latency percentiles','cancelAndRecovery':True}
    if args.output:args.output.parent.mkdir(parents=True,exist_ok=True);args.output.write_text(json.dumps(result,indent=2)+'\n')
    print(json.dumps(result,indent=2))
if __name__=='__main__':main()
