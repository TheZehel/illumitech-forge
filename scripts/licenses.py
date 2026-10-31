#!/usr/bin/env python3
"""Inventory the locked dependency graph and preserve downloaded license notices."""
import json,subprocess,shutil
from frontend_licenses import inventory as frontend_inventory
from pathlib import Path
root=Path(__file__).resolve().parents[1]
meta=json.loads(subprocess.check_output(['cargo','metadata','--locked','--offline','--format-version','1'],cwd=root))
dest=root/'THIRD_PARTY_LICENSES';dest.mkdir(exist_ok=True)
rows=[];missing=[]
for p in sorted(meta['packages'],key=lambda p:(p['name'],p['version'])):
 if p['source'] is None:continue
 source=Path(p['manifest_path']).parent
 licenses=[x for x in source.iterdir() if x.is_file() and (x.name.upper().startswith(('LICENSE','COPYING','NOTICE','UNLICENSE')))]
 if not licenses:
  extra=root/'third-party-extra'/p['name']
  if extra.is_dir():licenses=[x for x in extra.iterdir() if x.is_file()]
 sub=dest/(p['name']+'-'+p['version']);sub.mkdir(exist_ok=True)
 for file in licenses:shutil.copyfile(file,sub/file.name)
 rows.append({'name':p['name'],'version':p['version'],'license':p['license'],'source':p['source'],'notices':[x.relative_to(dest).as_posix() for x in sorted(sub.iterdir())]})
 if not p['license'] or not licenses:missing.append(p['name'])
frontend=frontend_inventory(root,dest)
missing.extend(frontend['missing_required_notices'])
(root/'docs/dependencies.json').write_text(json.dumps({'schema_version':1,'dependencies':rows,'missing_notices':missing,'frontend':frontend},indent=2)+'\n')
print(json.dumps({'dependencies':len(rows),'missing_notices':missing}))
raise SystemExit(bool(missing))
