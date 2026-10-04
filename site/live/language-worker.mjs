// Static analysis only: no evaluation or resource access.
import init, {language_query,color_convert,color_presentations} from './wasm/laymesh_wasm.js';
const ready=init();
self.onmessage=async({data})=>{try{await ready;self.postMessage({id:data.id,result:JSON.parse(data.method==='colorConvert'?color_convert(JSON.stringify(data.params)):data.method==='colorPresentations'?color_presentations(JSON.stringify(data.files),data.file,data.params.from,data.params.to,JSON.stringify(data.params.rgba)):language_query(JSON.stringify(data.files),data.file,data.method,data.offset||0,data.locale||'en'))});}catch(error){self.postMessage({id:data.id,error:String(error)});}};
