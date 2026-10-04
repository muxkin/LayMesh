// Node uses the existing native CLI; there is no separate Node layout engine.
import {execFileSync} from 'node:child_process';
import {fileURLToPath} from 'node:url';
const binary=process.env.LAYMESH_CLI||fileURLToPath(new URL('../../target/release/laymesh',import.meta.url));
const source=process.argv[2]||fileURLToPath(new URL('../manual/line.lay',import.meta.url));
const output=process.argv[3]||'figure.svg';
execFileSync(binary,['validate',source],{stdio:'inherit'});
execFileSync(binary,['render',source,'-o',output],{stdio:'inherit'});
