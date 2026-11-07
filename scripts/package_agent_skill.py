#!/usr/bin/env python3
"""Build reproducible skill/plugin candidates. Never installs or submits anything."""
import argparse
import json
import re
import tomllib
from pathlib import Path
from package import write_archive

ROOT = Path(__file__).resolve().parents[1]
FILES = ('SKILL.md', 'agents/openai.yaml', 'references/setup.md', 'references/development.md',
         'references/continuity.md', 'references/operations.md', 'references/feature-request.json')

def package(root, output):
    version = tomllib.loads((root / 'Cargo.toml').read_text())['workspace']['package']['version']
    manifest = json.loads((root / 'release/agent-plugin.json').read_text())
    if manifest['version'] != version:
        raise ValueError('plugin/workspace versions differ')
    skill = root / 'skills/illumitech-forge'
    body = (skill / 'SKILL.md').read_text()
    if not re.search(r'^name: illumitech-forge$', body, re.M):
        raise ValueError('unexpected skill identity')
    for reference in re.findall(r'\]\((references/[^)]+)\)', body):
        if reference not in FILES:
            raise ValueError('missing packaged reference: ' + reference)
    if output.exists():
        raise ValueError('use a fresh output directory; existing artifacts are preserved')
    output.mkdir(parents=True)
    archives = []
    for kind in ('skill', 'plugin'):
        prefix = 'illumitech-forge/' if kind == 'skill' else 'skills/illumitech-forge/'
        files = {prefix + name: skill / name for name in FILES}
        files['LICENSE'] = root / 'LICENSE'
        if kind == 'plugin':
            files['plugin.json'] = root / 'release/agent-plugin.json'
        archive = output / f'illumitech-forge-v{version}-{kind}.zip'
        digest = write_archive(archive, files)
        archives.append({'file': archive.name, 'sha256': digest})
    (output / 'SHA256SUMS').write_text(''.join(f"{a['sha256']}  {a['file']}\n" for a in archives))
    return {'schema_version': 1, 'version': version, 'status': 'candidate', 'archives': archives,
            'host_acceptance': 'not implied by packaging', 'published': False}

if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output', required=True, type=Path)
    args = parser.parse_args()
    print(json.dumps(package(ROOT, args.output)))
