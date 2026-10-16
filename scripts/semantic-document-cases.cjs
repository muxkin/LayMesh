// Shared, deterministic edits for offline and VS Code document benchmarks.
exports.names=['lcss-heading','lcss-curves','laydoc-name','laydoc-reorder','laydoc-add-remove'];
exports.change=(kind,i,css,doc)=>{
 const color=`rgb(${30+i},${80+i},${120+i})`,rgba=[30+i,80+i,120+i].map(n=>n/255).concat(1);
 if(kind==='lcss-heading')return{file:'theme.lcss',source:css.replace(/(\.heading\s*\{[^}]*color:)#[0-9a-f]+/,`$1${color}`),rgba};
 if(kind==='lcss-curves')return{file:'theme.lcss',source:css+`\n.strand { border-color:${color} !important; }\n`,rgba};
 const lines=doc.trimEnd().split('\n'),pages=lines.filter(l=>l.startsWith('doc.add(')),header=lines.filter(l=>!l.startsWith('doc.add('));
 if(kind==='laydoc-name'){const name=`Measured document ${i}`;return{file:'storyboard.laydoc',source:doc.replace(/document\(name="[^"]*"/,`document(name="${name}"`),name,ids:pages.map(p=>p.match(/id="([^"]+)"/)[1])};}
 if(kind==='laydoc-reorder'){
  const pool=[...pages],ordered=[];let n=i+1;
  while(pool.length){const at=n%pool.length;n=Math.floor(n/pool.length);ordered.push(pool.splice(at,1)[0]);}
  return{file:'storyboard.laydoc',source:[...header,...ordered].join('\n')+'\n',ids:ordered.map(p=>p.match(/id="([^"]+)"/)[1])};
 }
 if(kind==='laydoc-add-remove'){
  const extra=i%2===0?`doc.add(page(src="pages/02-seed.lay",id="extra-${i}",name="Extra ${i}"))\n`:'';
  return{file:'storyboard.laydoc',source:doc+extra,operation:extra?'add':'remove',ids:[...pages.map(p=>p.match(/id="([^"]+)"/)[1]),...(extra?[`extra-${i}`]:[])]};
 }
 throw Error('Unknown document benchmark case: '+kind);
};
