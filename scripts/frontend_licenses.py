"""Inventory the exact frontend lock; copy notices only from local installed dependencies.

No registry lookup, package execution or license text fabrication. Optional foreign-platform
binaries are not distributed; their lock metadata is retained with explicit notice coverage.
"""
import hashlib
import json
import shutil
from pathlib import Path


def inventory(root, destination):
    ui = root / 'apps/forge-ui'
    lock = json.loads((ui / 'package-lock.json').read_text())
    rows, missing = [], []
    for location, package in sorted(lock['packages'].items()):
        if not location:
            continue
        name = location.rsplit('node_modules/', 1)[-1]
        source = ui / location
        files = sorted(p for p in source.glob('*') if p.is_file() and
                       p.name.upper().startswith(('LICENSE', 'COPYING', 'NOTICE', 'UNLICENSE', 'THIRDPARTYNOTICE')))
        basis = 'installed package notice'
        # Binary-only platform packages share the parent project's upstream license.
        # Identify this explicitly instead of inventing per-platform downloaded notices.
        if not files and name.startswith(('@esbuild/', '@rollup/rollup-')):
            parent = 'esbuild' if name.startswith('@esbuild/') else 'rollup'
            files = sorted(p for p in (ui / 'node_modules' / parent).glob('LICENSE*') if p.is_file())
            basis = 'upstream family notice from installed ' + parent
        target = destination / ('frontend-' + name.replace('/', '__') + '-' + package['version'])
        target.mkdir(exist_ok=True)
        for file in files:
            shutil.copyfile(file, target / file.name)
        if not files:
            basis = 'not installed or distributed; lock metadata only'
            if source.exists() or not package.get('optional'):
                missing.append(name)
        if not package.get('license'):
            missing.append(name)
        rows.append({
            'name': name, 'version': package['version'], 'license': package.get('license'),
            'source': package.get('resolved'), 'integrity': package.get('integrity'),
            'scope': 'build/test' if package.get('dev') else 'runtime',
            'optional': package.get('optional', False), 'installed': source.is_dir(),
            'notice_basis': basis,
            'notices': [str((target / p.name).relative_to(destination)) for p in files],
        })
    assets = []
    for relative in ['docs/assets/logo.webp', 'docs/assets/cover.webp', 'docs/assets/memory.webp']:
        path = root / relative
        assets.append({'path': relative, 'sha256': hashlib.sha256(path.read_bytes()).hexdigest(),
                       'provenance': 'docs/assets/provenance.json', 'owner': 'Illumitech'})
    return {'lock_sha256': hashlib.sha256((ui / 'package-lock.json').read_bytes()).hexdigest(),
            'dependencies': rows, 'missing_required_notices': sorted(set(missing)), 'assets': assets}
