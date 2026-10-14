// Regression entry point: display configuration must not trigger automatic
// presenter mode, permissions or window movement.
const {spawnSync}=require('node:child_process'),path=require('node:path');
const result=spawnSync(process.execPath,[path.join(__dirname,'test-presenter-startup.cjs'),...process.argv.slice(2)],{stdio:'inherit',env:{...process.env,LAYMESH_STARTUP_CASES:'single,extended,denied,unsupported,mirrored,manual'}});
process.exit(result.status??1);
