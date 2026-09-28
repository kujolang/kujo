from pathlib import Path
import subprocess,os,time,json
logs=Path('/tmp/kujo16-rc-gates'); root=Path.cwd()
while not (logs/'build.exit').exists(): time.sleep(3)
assert (logs/'build.exit').read_text().strip()=='0'
with (logs/'clean-install.log').open('w') as f:
 r=subprocess.run(['python3','/tmp/kujo16-clean-install.py'],stdout=f,stderr=subprocess.STDOUT)
(logs/'clean-install.exit').write_text(str(r.returncode)+'\n');assert r.returncode==0
m=json.loads((logs/'clean-install.json').read_text()); print(m.keys(),flush=True)
binary=str(Path(m['install_root'])/'bin'/'kujo')
assert Path(binary).exists(), binary
env=dict(os.environ,KUJO_BIN=binary,KUJO=binary,DISPATCH_OFFLINE_FIXTURE='true',WORKCELL_TEST_KUJO_VERSION='1.6.0')
commands=[('dispatch',root.parent/'dispatch',['bash','scripts/run_release_gate.sh']),('adopter',root.parent/'dispatch',['python3','tests/adopter_pilot_integration.py']),('workcell',root.parent/'workcell',['bash','tests/run.sh']),('workcell-assurance',root.parent/'workcell',['bash','tests/effect_assurance.sh'])]
(logs/'downstream-commands.json').write_text(json.dumps([{'name':n,'cwd':str(p),'argv':c,'binary':binary} for n,p,c in commands],indent=2))
for n,p,c in commands:
 with (logs/(n+'.log')).open('w') as f: r=subprocess.run(c,cwd=p,env=env,stdout=f,stderr=subprocess.STDOUT)
 (logs/(n+'.exit')).write_text(str(r.returncode)+'\n'); print(n,r.returncode,flush=True)
