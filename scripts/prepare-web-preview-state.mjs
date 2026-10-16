#!/usr/bin/env node
// Establish actual reads once while packaging, so the user's first local edit
// does not have to compile every other page. Preserve all artwork entries.
import fs from 'node:fs';
import assert from 'node:assert/strict';
import {webPayload,webUnzip,webZip,webUniversal} from '../site/live/web-archive.mjs';
import {rebuildPreviewState} from './web-preview-state-package.mjs';
const [input,output]=process.argv.slice(2);assert(input&&output&&input!==output,'Usage: INPUT NEW_OUTPUT');
const files=webUnzip(webPayload(fs.readFileSync(input))),decode=b=>new TextDecoder().decode(b);
const records=await rebuildPreviewState(files);
files['entries.json']=new TextEncoder().encode(JSON.stringify(Object.keys(files)));
const packed=Buffer.from(await webUniversal(webZip(files),decode(files['runtime/bootstrap.js'])).arrayBuffer());fs.writeFileSync(output,packed,{flag:'wx'});
console.log(JSON.stringify({output,pages:records}));
