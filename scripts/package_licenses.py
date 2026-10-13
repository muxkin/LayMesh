"""Package the native license inventory without byte-identical duplicate files."""
import hashlib
import json
import shutil


def package(root, destination):
    source=root/'release/licenses'
    inventory=json.loads((source/'manifest.json').read_text())
    # Prefer filenames already cited in the human-readable third-party notice.
    preferred=['Matplotlib-3.11.2-LICENSE','GL-Transitions.txt',
               'css-animate-LICENSE.txt','css-bootstrap-LICENSE.txt','css-open-props-LICENSE.txt']
    canonical={}
    aliases={}
    used={r['file'] for p in inventory for r in p['texts']} | {'GL-Transitions.txt'}
    paths=sorted((source/name for name in used),
                 key=lambda p:(p.name not in preferred,p.name))
    destination.mkdir(parents=True,exist_ok=True)
    # Only this generated packaging directory is cleared; repository originals
    # and pinned source hashes are left intact.
    for p in destination.iterdir():
        if p.is_file(): p.unlink()
    for path in paths:
        digest=hashlib.sha256(path.read_bytes()).hexdigest()
        if digest not in canonical:
            canonical[digest]=path.name
            shutil.copyfile(path,destination/path.name)
        aliases[path.name]=canonical[digest]
    for record in inventory:
        for ref in record['texts']:
            original=ref['file'];ref['file']=aliases[original]
            if original!=ref['file']:ref['original_file']=original
            assert hashlib.sha256((destination/ref['file']).read_bytes()).hexdigest()==ref['sha256']
    (destination/'manifest.json').write_text(json.dumps(inventory,indent=2)+'\n')
    (destination/'aliases.json').write_text(json.dumps({k:v for k,v in aliases.items() if k!=v},indent=2)+'\n')
