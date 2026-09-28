from pathlib import Path
import time,subprocess,json,hashlib,tempfile
logs=Path('/tmp/kujo16-rc-gates');dist=Path('/tmp/kujo-1.6.0-rc-44af277');source='44af277848173664f72ca85f2a1b3b98d634ecdd'
while not (logs/'build.exit').exists():time.sleep(3)
assert (logs/'build.exit').read_text().strip()=='0'
commands=[]
with (logs/'npm-artifacts.log').open('w') as f:
 def run(args,cwd=None):
  commands.append(args);r=subprocess.run(args,cwd=cwd,text=True,stdout=subprocess.PIPE,stderr=subprocess.STDOUT);f.write(r.stdout);f.flush();assert r.returncode==0,(args,r.stdout);return r.stdout.strip()
 run(['node','npm/scripts/pack-platform.js','--target','darwin-x64','--binary',str(dist/'native/kujo'),'--version','1.6.0','--git-commit',source,'--output',str(dist/'npm')])
 run(['node','npm/scripts/pack-runtime.js','--version','1.6.0','--output',str(dist/'npm')])
 archives=sorted((dist/'npm').glob('*.tgz'));assert len(archives)==2
 consumer=Path(tempfile.mkdtemp(prefix='kujo16-npm-consumer-'));(consumer/'package.json').write_text('{"name":"kujo16-private-consumer","private":true,"version":"0.0.0"}\n')
 run(['npm','install','--offline','--ignore-scripts','--no-audit','--no-fund','--omit=optional',*[str(a) for a in archives]],consumer)
 assert run([str(consumer/'node_modules/.bin/kujo'),'--version'],consumer)=='kujo 1.6.0'
 result={'ok':True,'source_commit':source,'archives':{p.name:hashlib.sha256(p.read_bytes()).hexdigest() for p in archives},'consumer':str(consumer),'lock_sha256':hashlib.sha256((consumer/'package-lock.json').read_bytes()).hexdigest(),'node':run(['node','--version']),'npm':run(['npm','--version']),'commands':commands}
 (logs/'npm-artifacts.json').write_text(json.dumps(result,indent=2)+'\n')
(logs/'npm-artifacts.exit').write_text('0\n')
