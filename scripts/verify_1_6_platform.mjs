// Release-only verification: execute downloaded binaries, never compile source.
import fs from 'node:fs';
import path from 'node:path';
import crypto from 'node:crypto';
import {execFileSync,spawnSync} from 'node:child_process';
const platform=process.argv[2];
if(!['linux-x64','linux-arm64','macos-x64','macos-arm64','windows-x64'].includes(platform))throw Error('unsupported platform');
const sha=b=>crypto.createHash('sha256').update(b).digest('hex');
const ext=platform==='windows-x64'?'zip':'tar.gz', name=`kujo-v1.6.0-${platform}.${ext}`;
const archive=path.resolve('incoming',name), expected=fs.readFileSync(archive+'.sha256','utf8').replace(/^\uFEFF/,'').trim().split(/\s+/)[0];
if(!/^[a-f0-9]{64}$/.test(expected)||sha(fs.readFileSync(archive))!==expected)throw Error('archive checksum mismatch');
const isolated=fs.mkdtempSync(path.resolve('isolated-'));fs.mkdirSync(path.join(isolated,'bin'));
if(ext==='zip')execFileSync('powershell.exe',['-NoProfile','-Command',`Expand-Archive -LiteralPath '${archive.replaceAll("'","''")}' -DestinationPath '${path.join(isolated,'bin').replaceAll("'","''")}'`]);
else execFileSync('tar',['-xzf',archive,'-C',path.join(isolated,'bin')]);
const binary=path.join(isolated,'bin',platform==='windows-x64'?'kujo.exe':'kujo');
const results=[];
function run(args){const r=spawnSync(binary,args,{cwd:isolated,encoding:'utf8',timeout:120000,env:{...process.env,KUJO_PATH:'',KUJO_MODULE_PATH:'',KUJO_BIN:binary}});results.push({args,status:r.status,stdout:r.stdout,stderr:r.stderr});if(r.error||r.status!==0)throw Error(JSON.stringify(results.at(-1)));return r.stdout.replaceAll('\r\n','\n');}
if(run(['--version']).trim()!=='kujo 1.6.0')throw Error('version mismatch');
const source=path.resolve('source');
function copied(name,text){const f=path.join(isolated,name);fs.writeFileSync(f,text);return f;}
for(const name of fs.readdirSync(path.join(source,'tests/fixtures/loop_return')).filter(n=>n.endsWith('.kujo'))){const f=copied(name,fs.readFileSync(path.join(source,'tests/fixtures/loop_return',name)));const vm=run(['run',f]), interpreter=run(['run','--interpreter',f]);if(vm!==interpreter)throw Error('loop parity mismatch '+name);if(name==='historical.kujo'&&vm!=='{"ok":true}\n')throw Error('historical regression');}
for(const name of ['vm_closure_adder','test_generators']){const f=copied(name+'.kujo',fs.readFileSync(path.join(source,'tests',name+'.kujo')));const expected=fs.readFileSync(path.join(source,'tests',name+'.out'),'utf8').replaceAll('\r\n','\n').trimEnd()+'\n';if(run(['run',f])!==expected)throw Error(name+' mismatch');}
if(run(['run',copied('hello.kujo','print(42)\n')])!=='42\n')throw Error('hello mismatch');
const task='async func add(a,b) { return a+b } print(await add(4,5)) func work() { return 21 } let h := spawn_task(work) let a := await await_task(h) let b := await await_task(h) print(a+b)\n';
if(run(['run',copied('async-task.kujo',task)])!=='9\n42\n')throw Error('task mismatch');
fs.writeFileSync('verification.json',JSON.stringify({ok:true,platform,source_commit:'44af277848173664f72ca85f2a1b3b98d634ecdd',archive:name,archive_sha256:expected,binary_sha256:sha(fs.readFileSync(binary)),source_tree_fallback:false,results},null,2)+'\n');
console.log(platform,results.length,'checks passed');
