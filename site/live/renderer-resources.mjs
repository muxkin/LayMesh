// VS Code can route a blob worker's local-resource requests through another
// webview's message port. Read in the owning window and transfer the bytes.
async function readBytes(url,label){
 const response=await fetch(url);
 if(!response.ok)throw Error(`${label} loading failed (${response.status})`);
 return response.arrayBuffer();
}
export async function loadRendererRuntime(base){
 const [wasmBytes,skiaBytes]=await Promise.all(['wasm','skia'].map(async folder=>{
  const response=await fetch(new URL(folder+'/manifest.json',base));
  if(!response.ok)throw Error(`${folder} manifest loading failed (${response.status})`);
  const manifest=await response.json();
  return readBytes(new URL(folder+'/'+manifest.wasm,base),folder+' WASM');
 }));
 return {wasmBytes,skiaBytes};
}
export async function loadDrawingResources(versions,known){
 const resources=new Map();
 for(const version of versions)for(const resource of version.resources||[])if(!known.has(resource.id))resources.set(resource.id,resource);
 return Promise.all([...resources.values()].map(async resource=>({id:resource.id,bytes:await readBytes(resource.uri,'Drawing resource')})));
}
