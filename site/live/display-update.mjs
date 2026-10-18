// Paths address the previous baseline. Copy only changed ancestors; unchanged
// geometry, siblings and resource buffers retain their JavaScript identities.
export function applyDisplayUpdate(previous, update) {
  if (!previous || update.schemaVersion !== previous.schemaVersion ||
      ![update.width, update.height].every(n => Number.isFinite(n) && n > 0))
    throw Error('Invalid drawing update');
  const changes = update.nodes || [], properties = update.properties || [], structure = update.structure || [];
  if (update.roots && (changes.length || properties.length || structure.length)) throw Error('Ambiguous drawing roots');
  const prefix = (a,b) => a.length <= b.length && a.every((n,i) => n === b[i]);
  const at = path => {
    if (!path.length) throw Error('Empty drawing path');
    let nodes = previous.nodes, node;
    for (let i=0;i<path.length;i++) {
      if (!Number.isSafeInteger(path[i]) || path[i]<0 || !(node=nodes?.[path[i]])) throw Error('Invalid drawing path');
      nodes = node.kind.kind === 'group' ? node.kind.children : null;
    }
    return node;
  };
  for (let i=0;i<changes.length;i++) {
    at(changes[i].path);
    if (changes.some((b,j) => j!==i && prefix(changes[i].path,b.path))) throw Error('Overlapping drawing changes');
  }
  const common = new Set(['name','transform','opacity','effects','semantic','hint']);
  const fields = {group:['clip','mask'],path:['path','fill','stroke','even_odd'],image:['resource','rect','nearest'],glyphs:['resource','text','size','glyphs','outlines','color','family','weight','italic','width','ascent','descent']};
  const seen = new Set();
  for (const p of properties) {
    const node=at(p.path), key=p.path.join('/');
    if (seen.has(key) || changes.some(n => prefix(n.path,p.path))) throw Error('Overlapping drawing properties');
    seen.add(key);
    for (const key of Object.keys(p.values)) if (!common.has(key) && !(key.startsWith('kind.') && fields[node.kind.kind]?.includes(key.slice(5)))) throw Error('Unknown drawing property');
  }
  seen.clear();
  for (const s of structure) {
    const node=s.parent.length?at(s.parent):null, nodes=node?.kind.kind==='group'?node.kind.children:node?null:previous.nodes,key=s.parent.join('/');
    if (!nodes || seen.has(key) || changes.some(n => prefix(n.path,s.parent)) ||
        !Number.isSafeInteger(s.start) || !Number.isSafeInteger(s.delete) || s.start<0 || s.delete<0 || s.start+s.delete>nodes.length)
      throw Error('Invalid drawing splice');
    seen.add(key);const used=new Set();
    for (const item of s.insert) {
      if ('existing' in item) {
        const n=item.existing;
        if (!Number.isSafeInteger(n) || n<s.start || n>=s.start+s.delete || used.has(n)) throw Error('Invalid retained child');
        used.add(n);
      } else if (!item.new?.kind) throw Error('Invalid inserted child');
    }
  }
  const list={...previous,media:update.media,animation:update.animation,pageTransition:update.pageTransition,width:update.width,height:update.height,nodes:update.roots||previous.nodes.slice(),resources:update.resources||previous.resources};
  const copied = new Set();
  const copy = path => {
    let nodes=list.nodes,node;
    path.forEach((index,i) => {
      const key=path.slice(0,i+1).join('/');
      if (!copied.has(key)) {node={...nodes[index]};if(node.kind.kind==='group')node.kind={...node.kind,children:node.kind.children.slice()};nodes[index]=node;copied.add(key);}
      else node=nodes[index];
      nodes=node.kind.children;
    });
    return node;
  };
  for (const p of properties) {
    const node=copy(p.path);let kind=false;
    for(const [key,value] of Object.entries(p.values)) {
      if(key.startsWith('kind.')){if(!kind){node.kind={...node.kind};kind=true;}node.kind[key.slice(5)]=value;}
      else node[key]=value;
    }
  }
  for (const n of changes) {
    const nodes=n.path.length===1?list.nodes:copy(n.path.slice(0,-1)).kind.children;
    nodes[n.path.at(-1)]=n.node;
  }
  for (const s of [...structure].sort((a,b)=>b.parent.length-a.parent.length)) {
    const nodes=s.parent.length?copy(s.parent).kind.children:list.nodes;
    const insert=s.insert.map(item=>'existing' in item?nodes[item.existing]:item.new);
    nodes.splice(s.start,s.delete,...insert);
  }
  return list;
}
