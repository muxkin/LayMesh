// Bounded histograms retain first-play spikes without storing every frame or
// conflating CPU submission/completion with physical display presentation.
export function recordFrame(metrics,key,result,{time,requestMs,intervalMs,running}){
 if(!running||result.unchanged)return;
 const rows=metrics.playback??=[];let row=rows.at(-1);
 if(row?.key!==key){row={key,frames:0,skipped:0,layerBuilds:0,budgetMisses:0,spikes:[],histograms:{sampleMs:Array(202).fill(0),drawMs:Array(202).fill(0),requestMs:Array(202).fill(0),completionIntervalMs:Array(202).fill(0)}};rows.push(row);if(rows.length>512){rows.shift();metrics.droppedPlaybackRecords=(metrics.droppedPlaybackRecords||0)+1;}}
 if(result.skipped){row.skipped++;return;}row.frames++;row.layerBuilds+=result.layerCacheBuilds||0;row.budgetMisses+=result.layerCacheBudgetMisses||0;
 for(const [name,value]of Object.entries({sampleMs:result.sampleMs,drawMs:result.rasterMs,requestMs,completionIntervalMs:intervalMs}))if(Number.isFinite(value)){row.histograms[name][Math.min(201,Math.ceil(value))]++;row[name+'Max']=Math.max(row[name+'Max']||0,value);}
 if(requestMs>16.7||result.layerCacheBuilds){row.spikes.push({time,requestMs,drawMs:result.rasterMs,layerBuilds:result.layerCacheBuilds});row.spikes.sort((a,b)=>b.requestMs-a.requestMs);row.spikes.length=Math.min(row.spikes.length,8);}
}
