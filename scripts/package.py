#!/usr/bin/env python3
"""Deterministic local candidate archive. No build, install, registry or remote action."""
import argparse
import gzip
import hashlib
import io
from pathlib import Path
import tarfile
import tomllib
import zipfile

ROOT = Path(__file__).resolve().parents[1]
TARGETS = ('aarch64-apple-darwin', 'x86_64-apple-darwin',
           'aarch64-unknown-linux-gnu', 'x86_64-unknown-linux-gnu', 'x86_64-pc-windows-msvc')


def write_archive(archive, files):
    """Normalize archive metadata, order and gzip header. Refuse symlinks/special files."""
    content = {}
    executable = set()
    for name, path in sorted(files.items()):
        if path.is_symlink() or not path.is_file():
            raise ValueError('archive input must be a regular file: ' + name)
        content[name] = path.read_bytes()
        if path.stat().st_mode & 0o111:
            executable.add(name)
    content['SHA256SUMS'] = ''.join(
        f'{hashlib.sha256(data).hexdigest()}  {name}\n' for name, data in content.items()
    ).encode()
    if archive.suffix == '.zip':
        with zipfile.ZipFile(archive, 'w', zipfile.ZIP_DEFLATED) as out:
            for name, data in sorted(content.items()):
                info = zipfile.ZipInfo(name, (1980, 1, 1, 0, 0, 0))
                info.compress_type = zipfile.ZIP_DEFLATED
                info.create_system = 3
                info.external_attr = (0o100755 if name in executable else 0o100644) << 16
                out.writestr(info, data)
    else:
        with archive.open('wb') as raw, gzip.GzipFile(fileobj=raw, mode='wb', filename='', mtime=0) as gz:
            with tarfile.open(fileobj=gz, mode='w') as out:
                for name, data in sorted(content.items()):
                    info = tarfile.TarInfo(name)
                    info.size = len(data)
                    info.mode = 0o755 if name in executable else 0o644
                    out.addfile(info, io.BytesIO(data))
    digest = hashlib.sha256(archive.read_bytes()).hexdigest()
    archive.with_name(archive.name + '.sha256').write_text(f'{digest}  {archive.name}\n')
    return digest


def package(root, target, binary=None, output=None, forge_binary=None):
    windows = target.endswith('windows-msvc')
    binary = binary or root / 'target' / target / ('release/illumitech-forge.exe' if windows else 'release/illumitech-forge')
    forge_name = 'forge.exe' if windows else 'forge'
    forge_binary = forge_binary or binary.with_name(forge_name)
    if not forge_binary.is_file():
        raise ValueError('missing native forge executable; build both binary targets')
    out = output or root / 'dist'
    out.mkdir(parents=True, exist_ok=True)
    version = tomllib.loads((root / 'Cargo.toml').read_text())['workspace']['package']['version']
    names = ['README.md', 'README.en.md', 'LICENSE', 'THIRD_PARTY_NOTICES', 'THIRD_PARTY_LICENSES', 'docs/dependencies.json',
             'docs/v1-candidate.md', 'docs/native-command.md', 'docs/terminal-chat.md', 'docs/workbench.md', 'docs/token-economy.md', 'docs/duration-estimates.md',
             'docs/quickstart.md', 'docs/operational-parity.md', 'docs/contracts.md', 'scripts/demo.py', 'scripts/agent-fixtures.py']
    if windows:
        names += ['scripts/install.ps1', 'scripts/uninstall.ps1', 'scripts/windows-native-pilot.ps1', 'examples/windows', 'docs/windows-portability.md']
    files = {'illumitech-forge.exe' if windows else 'illumitech-forge': binary, forge_name: forge_binary}
    for name in names:
        path = root / name
        for file in sorted(path.rglob('*')) if path.is_dir() else [path]:
            if file.is_symlink():
                raise ValueError('archive symlink refused')
            if file.is_file():
                files[file.relative_to(root).as_posix()] = file
    label = 'windows-candidate' if windows else 'candidate'
    archive = out / f'illumitech-forge-v{version}-{label}-{target}{".zip" if windows else ".tar.gz"}'
    digest = write_archive(archive, files)
    print(archive.name, digest)
    return archive


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('target', choices=TARGETS)
    parser.add_argument('--binary', type=Path, help='Explicit locally built artifact; caller verifies host/target')
    parser.add_argument('--output', type=Path)
    parser.add_argument('--forge-binary', type=Path, help='Native forge binary; defaults to sibling of --binary')
    args = parser.parse_args()
    package(ROOT, args.target, args.binary, args.output, args.forge_binary)
