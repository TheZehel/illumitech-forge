#!/usr/bin/env python3
import hashlib,tarfile,sys,tomllib,zipfile
from pathlib import Path
root=Path(__file__).resolve().parents[1];target=sys.argv[1]
assert target in ['aarch64-apple-darwin','x86_64-unknown-linux-gnu','x86_64-pc-windows-msvc']
windows=target.endswith('windows-msvc')
binary=root/'target'/target/('release/illumitech-forge.exe' if windows else 'release/illumitech-forge')
assert binary.is_file()
out=root/'dist';out.mkdir(exist_ok=True)
version=tomllib.loads((root/'Cargo.toml').read_text())['workspace']['package']['version']
names=['LICENSE','THIRD_PARTY_NOTICES','THIRD_PARTY_LICENSES','docs/dependencies.json',
       'docs/quickstart.md','docs/operational-parity.md','docs/contracts.md','scripts/demo.py','scripts/agent-fixtures.py']
if windows:
 archive=out/f'illumitech-forge-v{version}-windows-candidate-{target}.zip'
 names += ['scripts/install.ps1','scripts/uninstall.ps1','examples/windows','docs/windows-portability.md']
 files={'illumitech-forge.exe':binary}
 for name in names:
  path=root/name
  for file in (sorted(path.rglob('*')) if path.is_dir() else [path]):
   if file.is_file():files[file.relative_to(root).as_posix()]=file
 sums=''.join(f"{hashlib.sha256(file.read_bytes()).hexdigest()}  {name}\n" for name,file in sorted(files.items()))
 with zipfile.ZipFile(archive,'w',zipfile.ZIP_DEFLATED) as zip:
  for name,file in files.items():zip.write(file,name)
  zip.writestr('SHA256SUMS',sums)
else:
 archive=out/f'illumitech-forge-v{version}-{target}.tar.gz'
 with tarfile.open(archive,'w:gz') as tar:
  tar.add(binary,arcname='illumitech-forge')
  for name in names:tar.add(root/name,arcname=name)
with archive.open('rb') as stream:hash=hashlib.file_digest(stream,'sha256').hexdigest()
(out/(archive.name+'.sha256')).write_text(f'{hash}  {archive.name}\n')
print(archive.name,hash)
