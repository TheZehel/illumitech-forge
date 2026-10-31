#!/usr/bin/env python3
"""Seal current public source and selected verified artifacts locally; never publish.

The receipt hashes the selected tree, not just HEAD (the worktree can be dirty). Excluded
build/runtime directories follow Git ignores. A source hash cannot grant human acceptance.
"""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess
import tarfile
import tomllib
from package import write_archive
from public_readiness import scan, forbidden_path

ROOT = Path(__file__).resolve().parents[1]


def source_files(root):
    raw = subprocess.check_output(['git', 'ls-files', '-z', '--cached', '--others', '--exclude-standard'], cwd=root)
    files = {}
    for name in sorted(set(raw.decode().split('\0')) - {''}):
        path = root / name
        if path.is_symlink() or (path.exists() and not path.is_file()):
            raise ValueError('nonregular source input: ' + name)
        if not path.exists():
            continue  # Intentional worktree deletion, never resurrect committed content.
        if forbidden_path(name) or scan(path.read_bytes()):
            raise ValueError('source privacy review required: ' + name + ' (content withheld)')
        files[name] = path
    return files


def seal(root, output, artifacts):
    root = root.resolve()
    output = output.resolve()
    if output.exists():
        raise ValueError('candidate output must be fresh')
    files = source_files(root)
    version = tomllib.loads((root / 'Cargo.toml').read_text())['workspace']['package']['version']
    inventory = [{'path': name, 'sha256': hashlib.sha256(path.read_bytes()).hexdigest(),
                  'bytes': path.stat().st_size, 'executable': bool(path.stat().st_mode & 0o111)} for name, path in files.items()]
    digest = hashlib.sha256(json.dumps(inventory, sort_keys=True, separators=(',', ':')).encode()).hexdigest()
    names = set()
    for path in artifacts:
        if path.name in names or path.is_symlink() or not path.is_file():
            raise ValueError('invalid or colliding artifact')
        names.add(path.name)
    output.mkdir(parents=True)
    archive = output / f'illumitech-forge-v{version}-candidate-source.tar.gz'
    write_archive(archive, files)
    with tarfile.open(archive) as packed:
        for item in inventory:
            member = packed.getmember(item['path'])
            if (hashlib.sha256(packed.extractfile(member).read()).hexdigest() != item['sha256']
                    or bool(member.mode & 0o111) != item['executable']):
                raise ValueError('source changed while sealing; discard incomplete candidate')
    entries = {archive.name: hashlib.sha256(archive.read_bytes()).hexdigest()}
    # Copy bytes into this receipt's boundary; subsequent source mutation is detectable.
    for path in artifacts:
        data = path.read_bytes()
        (output / path.name).write_bytes(data)
        entries[path.name] = hashlib.sha256(data).hexdigest()
    receipt = {'schema_version': 1, 'version': version, 'status': 'sealed_local_candidate',
               'source_tree_sha256': digest, 'source_inventory': inventory,
               'artifacts': entries, 'publication': 'not_performed',
               'operational_acceptance': 'pending independent final-source gates and native pilots',
               'provenance': 'current worktree bytes, not a clean commit or signed publisher attestation'}
    (output / 'candidate.json').write_text(json.dumps(receipt, indent=2) + '\n')
    entries['candidate.json'] = hashlib.sha256((output / 'candidate.json').read_bytes()).hexdigest()
    (output / 'SHA256SUMS').write_text(''.join(f'{sha}  {name}\n' for name, sha in sorted(entries.items())))
    return {k: v for k, v in receipt.items() if k != 'source_inventory'}


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--artifact', type=Path, action='append', default=[])
    args = parser.parse_args()
    print(json.dumps(seal(ROOT, args.output, args.artifact)))
