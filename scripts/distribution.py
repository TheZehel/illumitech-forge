#!/usr/bin/env python3
"""Local source-package verification. Never uploads. Python 3.11+, Cargo 1.94."""
import argparse
import hashlib
import json
import re
import shutil
from pathlib import Path, PurePosixPath, PureWindowsPath
import subprocess
import tarfile
import tempfile

ROOT = Path(__file__).resolve().parents[1]
ORDER = tuple('illumitech-forge-' + n for n in ('domain', 'adapters', 'catalog', 'cli'))


def cargo(root, *args, capture=False):
    return subprocess.run(['cargo', *args], cwd=root, check=True, text=True,
                          stdout=subprocess.PIPE if capture else None).stdout


def inventory(root):
    meta = json.loads(cargo(root, 'metadata', '--no-deps', '--locked', '--offline',
                            '--format-version', '1', capture=True))
    packages = {p['name']: p for p in meta['packages'] if p['id'] in meta['workspace_members']}
    validate_graph(packages, ORDER)
    return packages


def validate_graph(packages, order):
    if set(packages) != set(ORDER) or set(order) != set(packages) or len(order) != len(packages):
        raise ValueError('incomplete package inventory/order')
    for name in order:
        p = packages[name]
        for key in ('description', 'repository', 'readme', 'license'):
            if not p.get(key):
                raise ValueError('missing package metadata: ' + key)
        for dep in p['dependencies']:
            if dep.get('path'):
                if dep['name'] not in packages or dep['req'] != '=' + packages[dep['name']]['version']:
                    raise ValueError('internal dependency must have exact matching version')
                if order.index(dep['name']) >= order.index(name):
                    raise ValueError('invalid publication order')


def validate_files(files, *, ui=False):
    required = {'Cargo.toml', 'README.md', 'LICENSE', 'THIRD_PARTY_NOTICES', 'dependencies.json'}
    if ui:
        required |= {'build.rs', 'ui-dist/index.html', 'ui-dist/logo.webp', 'ui-dist/THIRD_PARTY_LICENSES.txt'}
    if not required <= set(files):
        raise ValueError('missing required package files')
    for name in files:
        p = PurePosixPath(name)
        if p.is_absolute() or PureWindowsPath(name).drive or '\\' in name or '..' in p.parts or not (
            name in required | {'Cargo.toml.orig', 'Cargo.lock', '.cargo_vcs_info.json'}
            or p.parts[0] in {'src', 'tests', 'THIRD_PARTY_LICENSES'}
            or (ui and re.fullmatch(r'ui-dist/assets/[A-Za-z0-9_-]+\.(js|css)', name))
        ) or any(x.startswith('.env') or x in {'credentials.json', 'auth.json'} for x in p.parts):
            raise ValueError('forbidden package file (content withheld)')


def verify_notices(root, packages):
    expected = (root / 'docs/dependencies.json').read_bytes()
    for p in packages.values():
        directory = Path(p['manifest_path']).parent
        for name in ('LICENSE', 'THIRD_PARTY_NOTICES'):
            if (directory / name).read_bytes() != (root / name).read_bytes():
                raise ValueError('stale package license/notice')
        if (directory / 'dependencies.json').read_bytes() != expected:
            raise ValueError('stale dependency inventory')
        for notice in (root / 'THIRD_PARTY_LICENSES').rglob('*'):
            if notice.is_file() and (directory / notice.relative_to(root)).read_bytes() != notice.read_bytes():
                raise ValueError('missing or stale dependency notice')


def run(mode, output=None):
    if output is not None:
        output = output.resolve()
        if output.exists():
            raise ValueError('candidate output must be fresh; never overwrite prior packages')
    packages = inventory(ROOT)
    verify_notices(ROOT, packages)
    package_files = {}
    for name in ORDER:
        # Cargo lists native paths on Windows; archive members remain strictly POSIX.
        files = [entry.replace('\\', '/') for entry in cargo(
            ROOT, 'package', '-p', name, '--list', '--allow-dirty', '--offline', capture=True
        ).splitlines()]
        validate_files(files, ui=name == ORDER[-1])
        package_files[name] = files
    if mode == 'list':
        print(json.dumps({'package_order': ORDER, 'package_lists': 'passed', 'package_files': package_files, 'upload': 'not_performed'}))
        return
    with tempfile.TemporaryDirectory(prefix='forge-distribution-') as temp:
        temp = Path(temp)
        # Cargo 1.94's temporary registry may fail verification of unpublished siblings.
        # --no-verify only creates archives; the mandatory extracted build below is the proof.
        registry_dry_run = 'not_requested'
        if mode == 'dry-run':
            # Offline registry/auth lookup may fail even before packaging. Keep this gate pending.
            result = subprocess.run(['cargo', 'publish', '--workspace', '--dry-run', '--offline',
                                     '--allow-dirty', '--no-verify', '--target-dir', str(temp / 'target')],
                                    cwd=ROOT, capture_output=True)
            registry_dry_run = 'archive_only' if result.returncode == 0 else 'pending_offline_registry_check'
        cargo(ROOT, 'package', '--workspace', '--offline', '--allow-dirty', '--no-verify',
              '--target-dir', str(temp / 'target'))
        extracted = temp / 'extracted'
        extracted.mkdir()
        hashes = {}
        members = []
        for name in ORDER:
            stem = name + '-' + packages[name]['version']
            archive = temp / 'target/package' / (stem + '.crate')
            hashes[archive.name] = hashlib.sha256(archive.read_bytes()).hexdigest()
            with tarfile.open(archive) as source:
                files = []
                for member in source.getmembers():
                    parts = PurePosixPath(member.name).parts
                    if not parts or parts[0] != stem or not member.isfile():
                        raise ValueError('unexpected archive entry')
                    files.append('/'.join(parts[1:]))
                validate_files(files, ui=name == ORDER[-1])
                source.extractall(extracted, filter='data')
            members.append(stem)
        patches = '\n'.join(f'{name} = {{ path = "{stem}" }}' for name, stem in zip(ORDER, members))
        (extracted / 'Cargo.toml').write_text('[workspace]\nresolver = "2"\nmembers = ' + json.dumps(members) + '\n[patch.crates-io]\n' + patches + '\n')
        cargo(extracted, 'build', '--workspace', '--offline', '--target-dir', str(temp / 'build'))
        executable = temp / 'build/debug' / ('illumitech-forge.exe' if __import__('os').name == 'nt' else 'illumitech-forge')
        contracts = json.loads(subprocess.check_output([executable, '--json', 'contracts'], cwd=extracted))
        if contracts['contracts']['manifest'] != 1:
            raise ValueError('extracted CLI contract mismatch')
        short = executable.with_name('forge.exe' if __import__('os').name == 'nt' else 'forge')
        short_contracts = json.loads(subprocess.check_output([short, '--json', 'contracts'], cwd=extracted))
        if short_contracts != contracts or not subprocess.check_output([short, '--version'], text=True).startswith('forge '):
            raise ValueError('extracted native forge contract mismatch')
        cargo(extracted, 'test', '-p', ORDER[-1], '--lib',
              'standalone_package_embeds_all_referenced_ui_assets', '--offline',
              '--target-dir', str(temp / 'build'))
        if output is not None:
            output.mkdir(parents=True)
            for name in hashes:
                shutil.copyfile(temp / 'target/package' / name, output / name)
            (output / 'SHA256SUMS').write_text(''.join(f'{sha}  {name}\n' for name, sha in sorted(hashes.items())))
        print(json.dumps({'archive_sha256': hashes, 'extracted_build': 'passed',
                          'resolution': 'temporary local patches, not live registry',
                          'upload': 'not_performed', 'registry_ownership_auth': 'pending',
                          'cargo_publish_dry_run': registry_dry_run}))


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('mode', choices=['list', 'package', 'dry-run'])
    parser.add_argument('--output', type=Path, help='Retain verified archives in a fresh local directory')
    args = parser.parse_args()
    run(args.mode, args.output)
