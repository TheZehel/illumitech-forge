#!/usr/bin/env python3
"""Query OSV using public locked registry package names and versions only."""
import json,tomllib,urllib.request,sys
from pathlib import Path
root=Path(__file__).resolve().parents[1]
packages=[p for p in tomllib.loads((root/'Cargo.lock').read_text())['package'] if p.get('source','').startswith('registry+')]
request=urllib.request.Request('https://api.osv.dev/v1/querybatch',data=json.dumps({'queries':[{'package':{'name':p['name'],'ecosystem':'crates.io'},'version':p['version']} for p in packages]}).encode(),headers={'Content-Type':'application/json'})
with urllib.request.urlopen(request,timeout=60) as response:results=json.load(response)['results']
findings=[{'package':p['name'],'version':p['version'],'advisories':[v['id'] for v in r.get('vulns',[])]} for p,r in zip(packages,results) if r.get('vulns')]
report={'schema_version':1,'source':'https://api.osv.dev','checked_packages':len(packages),'findings':findings}
print(json.dumps(report,indent=2));sys.exit(bool(findings))
