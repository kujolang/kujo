from pathlib import Path
import subprocess,json,time
logs=Path('/tmp/kujo16-rc-gates')
while not (logs/'workcell-assurance.exit').exists():time.sleep(3)
commands=json.loads((logs/'commands.json').read_text())
for name in ['check','vm','interpreter','dual','targeted']:
 with (logs/(name+'.log')).open('w') as f:r=subprocess.run(commands[name],stdout=f,stderr=subprocess.STDOUT)
 (logs/(name+'.exit')).write_text(str(r.returncode)+'\n');assert r.returncode==0,name
(logs/'release-candidate.log').write_bytes((logs/'final-source-release-candidate.log').read_bytes())
(logs/'final-refresh.exit').write_text('0\n')
