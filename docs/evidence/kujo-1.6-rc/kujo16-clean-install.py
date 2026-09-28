import pathlib,subprocess,tarfile,tempfile,hashlib,json,os,gzip
ROOT=pathlib.Path('/Users/robertdevore/2026/Kujolang/kujo-repos/kujo')
DIST=pathlib.Path('/tmp/kujo-1.6.0-rc-44af277')
meta=json.loads((DIST/'artifacts.json').read_text());assert meta['source_commit']=='44af277848173664f72ca85f2a1b3b98d634ecdd'
root=pathlib.Path(tempfile.mkdtemp(prefix='kujo16-clean-'));(root/'bin').mkdir();(root/'home').mkdir();(root/'tests').mkdir()
for name,digest in meta['archives'].items():assert hashlib.sha256((DIST/name).read_bytes()).hexdigest()==digest
subprocess.run(['tar','-xzf',str(DIST/'kujo-v1.6.0-macos-x64.tar.gz'),'-C',str(root/'bin')],check=True)
binary=root/'bin/kujo';assert hashlib.sha256(binary.read_bytes()).hexdigest()==meta['binary_sha256']
raw=gzip.decompress((DIST/'kujo-v1.6.0-source.tar.gz').read_bytes())
expected=subprocess.check_output(['git','archive','--format=tar','--prefix=kujo-v1.6.0/',meta['source_commit']],cwd=ROOT);assert raw==expected
subprocess.run(['tar','-xzf',str(DIST/'kujo-v1.6.0-source.tar.gz'),'-C',str(root)],check=True)
source=root/'kujo-v1.6.0';assert json.loads((source/'release/kujo-1.6.0-rc.json').read_text())['version']=='1.6.0'
cases={'A_no_return':'1\n2\n','B_unconditional':'1\n9\n','C_matching':'1\n','D_no_match':'9\n','E_later_match':'2\n','F_nested_if':'2\n9\n','G_nested_loop':'4\n9\n','H_caller':'11\n','I_fold':'5\n9\n','J_break_continue':'2\n','historical':'{"ok":true}\n'}
for n,out in cases.items():(root/'tests'/f'{n}.kujo').write_bytes((source/'tests/fixtures/loop_return'/f'{n}.kujo').read_bytes());(root/'tests'/f'{n}.out').write_text(out)
for name in ['vm_closure_adder','test_generators']:
 for ext in ['kujo','out']:(root/'tests'/f'{name}.{ext}').write_bytes((source/'tests'/f'{name}.{ext}').read_bytes())
(root/'tests/simple.kujo').write_text('print(42)\n');(root/'tests/simple.out').write_text('42\n')
(root/'tests/async_task.kujo').write_text('async func add(a, b) { return a + b } print(await add(4, 5)) func work() { return 21 } let h := spawn_task(work) let a := await await_task(h) let b := await await_task(h) print(a + b)\n');(root/'tests/async_task.out').write_text('9\n42\n')
env={'PATH':'/usr/bin:/bin','HOME':str(root/'home')};results=[]
def run(args):
 r=subprocess.run([str(binary),*args],cwd=root,env=env,capture_output=True,text=True,timeout=180);results.append({'args':args,'exit':r.returncode,'stdout':r.stdout,'stderr':r.stderr});assert r.returncode==0,results[-1];return r.stdout
assert run(['--version']).strip()=='kujo 1.6.0'
for f in sorted((root/'tests').glob('*.kujo')):
 for mode in [[],['--interpreter']]:
  actual=run(['run',*mode,str(f)]); expected=f.with_suffix('.out').read_text().rstrip('\n')+'\n'
  assert actual==expected,(f.name,mode,actual,expected)
s=run(['test','--runtime','dual']);assert 'interpreter_fallback=0' in s and 'Passed 15/15' in s,s
value={'ok':True,'install_root':str(root),'binary':str(binary),'source_archive_exact_git_archive_match':True,'source_commit':meta['source_commit'],'binary_sha256':meta['binary_sha256'],'programs':15,'vm_interpreter_executions':30,'dual_fallback':0,'results':results}
pathlib.Path('/tmp/kujo16-rc-gates/clean-install.json').write_text(json.dumps(value,indent=2)+'\n');print(json.dumps({k:v for k,v in value.items() if k!='results'}))
