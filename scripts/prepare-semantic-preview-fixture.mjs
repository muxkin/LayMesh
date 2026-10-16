#!/usr/bin/env node
// Extract the benchmark's source/resources without rewriting the attachment.
import fs from 'node:fs';
import path from 'node:path';
import assert from 'node:assert/strict';
import {webPayload,webUnzip} from '../site/live/web-archive.mjs';
if(process.argv.length!==4)throw Error('Usage: node scripts/prepare-semantic-preview-fixture.mjs ATTACHMENT NEW_DIRECTORY');
const files=webUnzip(webPayload(fs.readFileSync(process.argv[2])));
const manifest=JSON.parse(new TextDecoder().decode(files['manifest.json']));
const out=path.resolve(process.argv[3]);
assert(!fs.existsSync(out),'Use a new fixture directory');fs.mkdirSync(out,{recursive:true});
for(const [name,key]of Object.entries(manifest.project.files)){
 assert(name.startsWith('/project/'),'Unexpected project path: '+name);
 const target=path.resolve(out,name.slice('/project/'.length));
 assert(target.startsWith(out+path.sep),'Project path leaves fixture directory');
 fs.mkdirSync(path.dirname(target),{recursive:true});fs.writeFileSync(target,files[key]);
}
for(const id of ['02-seed','06-editable']){
 const source=fs.readFileSync(path.join(out,'pages',id+'.lay'),'utf8');
 fs.writeFileSync(path.join(out,'benchmark-'+id+'.lay'),source.replaceAll('"../','"./'));
}
console.log(JSON.stringify({fixture:out,fontPolicy:'Native preview uses the locally installed named fonts; browser tests retain the attachment font bytes.'}));
