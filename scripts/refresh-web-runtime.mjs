#!/usr/bin/env node
// Replace only the runtime of an existing offline universal HTML package.
import fs from 'node:fs';
import {rebuildPreviewState} from './web-preview-state-package.mjs';
import path from 'node:path';
import {fileURLToPath} from 'node:url';
import {createHash} from 'node:crypto';
import assert from 'node:assert/strict';
import {webPayload,webUnzip,webZip,webUniversal} from '../site/live/web-archive.mjs';
const root=path.resolve(path.dirname(fileURLToPath(import.meta.url)),'..');
if(process.argv.length<4||process.argv.length>5)throw Error('Usage: node scripts/refresh-web-runtime.mjs INPUT OUTPUT [RUNTIME_ZIP]');
const input=path.resolve(process.argv[2]),output=path.resolve(process.argv[3]);
assert.notEqual(input,output,'Preserve the original attachment');
const runtimePath=path.resolve(process.argv[4]||path.join(root,'target/web-export/runtime.zip'));
const original=fs.readFileSync(input),runtimeBytes=fs.readFileSync(runtimePath);
const files=webUnzip(webPayload(original)),runtime=webUnzip(runtimeBytes);
const preserved=Object.entries(files).filter(([name])=>!name.startsWith('runtime/')&&!name.startsWith('licenses/')&&name!=='entries.json'&&name!=='preview-state.json');
for(const [name,bytes]of Object.entries(runtime))if(name.startsWith('runtime/')||name.startsWith('licenses/'))files[name]=bytes;
const dependencies=await rebuildPreviewState(files);
files['entries.json']=new TextEncoder().encode(JSON.stringify(Object.keys(files)));
const packed=Buffer.from(await webUniversal(webZip(files),new TextDecoder().decode(files['runtime/bootstrap.js'])).arrayBuffer());
const verified=webUnzip(webPayload(packed));
for(const [name,bytes]of preserved)assert.deepEqual(verified[name],bytes,'Changed project/artwork asset: '+name);
fs.mkdirSync(path.dirname(output),{recursive:true});fs.writeFileSync(output,packed,{flag:'wx'});
const sha256=value=>createHash('sha256').update(value).digest('hex');
console.log(JSON.stringify({input,output,originalSha256:sha256(original),outputSha256:sha256(packed),runtimeSha256:sha256(runtimeBytes),preservedEntries:preserved.length,dependencies,bytes:packed.length},null,2));
