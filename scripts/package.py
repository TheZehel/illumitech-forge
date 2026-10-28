#!/usr/bin/env python3
import hashlib,tarfile,sys
from pathlib import Path
root=Path(__file__).resolve().parents[1];target=sys.argv[1]
assert target in ['aarch64-apple-darwin','x86_64-unknown-linux-gnu']
binary=root/'target'/target/'release/illumitech-forge'
assert binary.is_file()
out=root/'dist';out.mkdir(exist_ok=True)
archive=out/f'illumitech-forge-v0.1.0-{target}.tar.gz'
with tarfile.open(archive,'w:gz') as tar:
 tar.add(binary,arcname='illumitech-forge')
 for name in ['LICENSE','THIRD_PARTY_NOTICES','THIRD_PARTY_LICENSES','docs/dependencies.json',
              'docs/quickstart.md','scripts/demo.py','scripts/agent-fixtures.py']:
  tar.add(root/name,arcname=name)
hash=hashlib.file_digest(archive.open('rb'),'sha256').hexdigest()
(out/(archive.name+'.sha256')).write_text(f'{hash}  {archive.name}\n')
print(archive.name,hash)
