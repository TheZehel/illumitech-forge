#!/usr/bin/env python3
"""Render/check current installation docs; historical release notes are immutable records."""
import argparse
import json
import re
from pathlib import Path
ROOT = Path(__file__).resolve().parents[1]
TARGETS = ('README.md', 'README.en.md', 'docs/quickstart.md', 'apps/forge-site/src/content.ts', 'apps/forge-site/build.mjs',
           'crates/forge-cli/README.md', 'crates/forge-domain/README.md',
           'crates/forge-adapters/README.md', 'crates/forge-catalog/README.md')
def render(root, write=False):
    current = json.loads((root / 'release/current.json').read_text())
    version = current['published']
    if not re.fullmatch(r'[1-9][0-9]*\.[0-9]+\.[0-9]+', version):
        raise ValueError('invalid published version')
    for name in TARGETS:
        template = root / 'release/doc-templates' / (name.replace('/', '__') + '.in')
        body = template.read_text().replace('{{published}}', version)
        if '{{published}}' in body:
            raise ValueError('unexpanded version')
        # Only current install commands/URLs; historical prose may name old versions.
        versions = re.findall(r'--version ([0-9]+\.[0-9]+\.[0-9]+)|releases/tag/v([0-9]+\.[0-9]+\.[0-9]+)', body)
        if any((a or b) != version for a, b in versions):
            raise ValueError('incoherent current installation reference: ' + name)
        target = root / name
        if write:
            target.write_text(body)
        elif target.read_text() != body:
            raise ValueError('stale generated current documentation: ' + name)
    if not (root / f'docs/release-v{version}.md').is_file():
        raise ValueError('missing published release notes')
    return {'status': 'passed', 'published_version': version, 'current_documents': list(TARGETS),
            'historical_scope': 'docs/release-v*.md, specs and historical evidence retain their original versions'}
if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--write', action='store_true')
    args = parser.parse_args()
    print(json.dumps(render(ROOT, args.write)))
