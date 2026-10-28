#!/usr/bin/env python3
"""Create a disposable example; evaluator and state stay outside the task repo."""
import hashlib,json,subprocess,sys
from pathlib import Path
root=Path(sys.argv[1]).resolve();root.mkdir(parents=True,exist_ok=False)
repo=root/'app';repo.mkdir()
subprocess.run(['git','init','-q',str(repo)],check=True)
subprocess.run(['git','-C',str(repo),'-c','user.name=Forge Demo','-c','user.email=demo@example.invalid','commit','--allow-empty','-qm','Initialize disposable example'],check=True)
(repo/'preserved.txt').write_text('keep\n')
evaluator=root/'evaluate.py';evaluator.write_text("import sys\nfrom pathlib import Path\np=Path(sys.argv[1])\nassert (p/'hello.txt').read_text() == 'Hello Forge\\n'\nassert (p/'preserved.txt').read_text() == 'keep\\n'\n")
manifest={'schema_version':1,'release_id':'hello-forge','repos':{'app':{'path':'app'}},'tasks':[{'id':'hello','repo':'app','description':'Create a greeting while preserving unrelated files.','executor':{'kind':'command','argv':['python3','-c',"from pathlib import Path; Path('hello.txt').write_text('Hello Forge\\n')"]},'acceptance':[{'kind':'gate','id':'greeting'}]}],'gates':[{'id':'greeting','argv':['python3','{manifest_dir}/evaluate.py','{repo}'],'inputs':{'evaluate.py':hashlib.sha256(evaluator.read_bytes()).hexdigest()}}]}
(root/'release.yaml').write_text(json.dumps(manifest,indent=2)+'\n')
print(root/'release.yaml')
