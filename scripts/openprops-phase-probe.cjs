// Diagnostic-only injection into the self-contained worker. The ordinary
// performance matrix never enables this probe or changes the packaged file.
module.exports=function installOpenPropsPhaseProbe(){
 const NativeBlob=globalThis.Blob;
 globalThis.Blob=class extends NativeBlob {
  constructor(parts,options){
   parts=parts.map(part=>{
    if(typeof part!=='string'||!part.includes('class PageTransitions')||!part.includes('class SkiaRenderer'))return part;
    const replace=(a,b)=>{if(!part.includes(a))throw Error('Phase probe does not match runtime: '+a.slice(0,60));part=part.replace(a,b);};
    replace('const draw=(p)=>{','const phaseTimings=[];const draw=(p)=>{const phaseStarted=performance.now();');
    replace('output.flush();s.onPresent?.();','const phaseDrawn=performance.now();output.flush();const phaseFlushed=performance.now();s.onPresent?.();phaseTimings.push({p,drawMs:phaseDrawn-phaseStarted,flushMs:phaseFlushed-phaseDrawn});');
    replace('return {transitionName:spec.name','return {transitionTimings:phaseTimings,transitionSamples:surface.sampleCnt?.(),transitionOpaque:opaque,transitionName:spec.name');
    replace('c.restore();if(!present&&this.surface.flushDrawing)this.surface.flushDrawing();else this.surface.flush();','c.restore();const phaseDrawn=performance.now();this.diagnosticCompositionMs=phaseDrawn-start;if(!present&&this.surface.flushDrawing)this.surface.flushDrawing();else this.surface.flush();this.diagnosticSurfaceFlushMs=performance.now()-phaseDrawn;');
    replace('effectDraws:this.effectDraws,effectPictureReplays:this.effectPictureReplays,resourceStats:', 'diagnosticCompositionMs:this.diagnosticCompositionMs,diagnosticSurfaceFlushMs:this.diagnosticSurfaceFlushMs,effectDraws:this.effectDraws,effectPictureReplays:this.effectPictureReplays,resourceStats:');
    replace('const metrics=()=>({tiles,drawMs,','const metrics=()=>({diagnosticShadowMaxMs:this.diagnosticShadowMaxMs||0,diagnosticMaskMaxMs:this.diagnosticMaskMaxMs||0,tiles,drawMs,');
    replace('let tiles=0,drawMs=0,gpuWaitMs=0,maxTaskMs=0,','this.diagnosticShadowMaxMs=0;this.diagnosticMaskMaxMs=0;let tiles=0,drawMs=0,gpuWaitMs=0,maxTaskMs=0,');
    replace('if(!mask){\n   const b=effect.source.getBounds()', 'if(!mask){\n   const phaseMaskStarted=performance.now();const b=effect.source.getBounds()');
    replace('this.effectMaskPeakBytes=Math.max(this.effectMaskPeakBytes||0,this.effectMaskBytes+w*h*15);','this.effectMaskPeakBytes=Math.max(this.effectMaskPeakBytes||0,this.effectMaskBytes+w*h*15);this.diagnosticMaskMaxMs=Math.max(this.diagnosticMaskMaxMs||0,performance.now()-phaseMaskStarted);');
    replace('drawEffect(c,node,id,states,p,effect){','drawEffect(c,node,id,states,p,effect){const phaseEffectStarted=performance.now();');
    replace('  c.restore();c.restore();\n }\n localContent','  c.restore();c.restore();this.diagnosticShadowMaxMs=Math.max(this.diagnosticShadowMaxMs||0,performance.now()-phaseEffectStarted);\n }\n localContent');
    globalThis.__phasePatchedWorkers=(globalThis.__phasePatchedWorkers||0)+1;return part;
   });
   super(parts,options);
  }
 };
};
