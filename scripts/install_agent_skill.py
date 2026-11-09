#!/usr/bin/env python3
"""Verify one standalone Forge skill ZIP; preview by default. Python 3.11+, stdlib only."""
import argparse
import contextlib
import hashlib
import io
import json
import os
from pathlib import Path
import re
import signal
import stat
import subprocess
import sys
import threading
import time
import zipfile

SKILL = 'illumitech-forge'
# Deliberately independent of the packager: this is the accepted standalone format.
RESOURCES = ('SKILL.md', 'agents/openai.yaml', 'references/setup.md',
             'references/development.md', 'references/continuity.md',
             'references/operations.md', 'references/feature-request.json')
MAX_ARCHIVE = 8 * 1024 * 1024
MAX_FILE = 1024 * 1024
MAX_TOTAL = 4 * 1024 * 1024
MAX_ENTRIES = 32
PROBE_LIMIT = 512 * 1024
PROBE_SECONDS = 10


class SetupError(ValueError):
    """A refused installation; diagnostic text never includes subprocess output."""


def verify_archive(archive, expected):
    if not re.fullmatch(r'[0-9a-fA-F]{64}', expected):
        raise SetupError('--sha256 must be the trusted whole-archive SHA-256 (64 hex digits)')
    # Nonblocking open lets us reject FIFOs/devices before attempting a read.
    fd = os.open(archive, os.O_RDONLY | getattr(os, 'O_NONBLOCK', 0))
    with os.fdopen(fd, 'rb') as source:
        if not stat.S_ISREG(os.fstat(source.fileno()).st_mode):
            raise SetupError('--archive must be a regular standalone skill ZIP')
        data = source.read(MAX_ARCHIVE + 1)
    if len(data) > MAX_ARCHIVE:
        raise SetupError('archive exceeds the 8 MiB bound')
    digest = hashlib.sha256(data).hexdigest()
    if digest != expected.lower():
        raise SetupError('whole-archive SHA-256 mismatch; obtain the matching trusted release')
    expected_files = {f'{SKILL}/{name}' for name in RESOURCES} | {'LICENSE', 'SHA256SUMS'}
    with zipfile.ZipFile(io.BytesIO(data)) as packed:
        entries = packed.infolist()
        if len(entries) > MAX_ENTRIES:
            raise SetupError('ZIP inventory exceeds the entry bound')
        names = set()
        total = 0
        for item in entries:
            name = item.filename
            mode = item.external_attr >> 16
            if (item.orig_filename != name or name not in expected_files or name in names
                    or item.is_dir() or item.external_attr & 0x10
                    or stat.S_IFMT(mode) not in (0, stat.S_IFREG)
                    or item.flag_bits & 1
                    or item.compress_type not in (zipfile.ZIP_STORED, zipfile.ZIP_DEFLATED)):
                raise SetupError('unsafe, duplicate or unexpected ZIP member; use the standalone skill ZIP')
            names.add(name)
            total += item.file_size
            if (item.file_size > MAX_FILE or item.compress_size > MAX_ARCHIVE
                    or total > MAX_TOTAL):
                raise SetupError('ZIP expansion exceeds the per-file or total bound')
        if names != expected_files:
            raise SetupError('incomplete standalone skill inventory (plugin archives are unsupported)')
        content = {}
        for item in entries:
            with packed.open(item) as member:
                body = member.read(MAX_FILE + 1)
            if len(body) != item.file_size or len(body) > MAX_FILE:
                raise SetupError('invalid ZIP member length')
            content[item.filename] = body
    sums = {}
    for line in content['SHA256SUMS'].decode('ascii').splitlines():
        match = re.fullmatch(r'([0-9a-fA-F]{64})  (.+)', line)
        if not match or match[2] in sums:
            raise SetupError('invalid or duplicate inner SHA256SUMS entry')
        sums[match[2]] = match[1].lower()
    if set(sums) != names - {'SHA256SUMS'}:
        raise SetupError('inner SHA256SUMS must cover every file exactly once')
    if any(hashlib.sha256(content[name]).hexdigest() != sha for name, sha in sums.items()):
        raise SetupError('inner SHA256SUMS mismatch')
    for name, body in content.items():
        if not body or '\x00' in body.decode('utf-8'):
            raise SetupError('skill resources must be nonempty UTF-8 text')
    entrypoint = content[f'{SKILL}/SKILL.md'].decode('utf-8')
    front = entrypoint.split('---\n', 2)
    if (len(front) != 3 or front[0] or
            re.findall(r'^name:\s*(.*?)\s*$', front[1], re.M) != [SKILL] or
            re.findall(r'^license:\s*(.*?)\s*$', front[1], re.M) != ['MIT']):
        raise SetupError('public skill identity must be illumitech-forge with MIT licensing')
    if 'MIT License' not in content['LICENSE'].decode('utf-8'):
        raise SetupError('missing MIT license')
    for link in re.findall(r'\]\((references/[^)]+)\)', entrypoint):
        if link not in RESOURCES:
            raise SetupError('entrypoint references an unshipped resource')
    request = json.loads(content[f'{SKILL}/references/feature-request.json'])
    if (not isinstance(request, dict) or type(request.get('schema_version')) is not int
            or request['schema_version'] != 1):
        raise SetupError('unsupported bundled Feature request')
    files = {name: content[f'{SKILL}/{name}'] for name in RESOURCES}
    files['LICENSE'] = content['LICENSE']
    return files, digest


def probe(argv, cwd, env=None):
    """Drain both pipes with fixed memory, deadline and process-group cleanup."""
    chunks = [bytearray(), bytearray()]
    overflow = threading.Event()
    with subprocess.Popen(argv, cwd=cwd, env=env, stdin=subprocess.DEVNULL,
                          stdout=subprocess.PIPE, stderr=subprocess.PIPE,
                          start_new_session=os.name == 'posix') as process:
        def read(pipe, buffer):
            try:
                while block := pipe.read(4096):
                    if len(buffer) + len(block) > PROBE_LIMIT:
                        overflow.set()
                        return
                    buffer.extend(block)
            finally:
                pipe.close()
        readers = [threading.Thread(target=read, args=(pipe, buffer), daemon=True)
                   for pipe, buffer in zip((process.stdout, process.stderr), chunks)]
        for reader in readers:
            reader.start()
        deadline = time.monotonic() + PROBE_SECONDS
        while time.monotonic() < deadline and not overflow.is_set():
            if process.poll() is not None and not any(t.is_alive() for t in readers):
                break
            overflow.wait(0.01)
        else:
            if os.name == 'posix':
                with contextlib.suppress(ProcessLookupError):
                    os.killpg(process.pid, signal.SIGKILL)
            else:
                process.kill()
            process.wait()
            for reader in readers:
                reader.join(timeout=0.1)
            raise SetupError('setup probe exceeded its time/output bound; check the selected executable')
        if process.returncode != 0:
            raise SetupError('setup probe failed; check the selected executable and its contracts')
    return bytes(chunks[0]).decode('utf-8').strip()


def explicit_cli(binary, root):
    # abspath is deliberate: never search PATH, even for a bare relative filename.
    binary = Path(os.path.abspath(binary))
    if not binary.is_file() or not os.access(binary, os.X_OK):
        raise SetupError('--forge-binary must select an existing executable; obtain a compatible Forge CLI')
    version = probe([str(binary), '--version'], root)
    match = re.fullmatch(r'(?:illumitech-forge|forge) (1\.(\d+)\.(\d+))(?:\+[0-9A-Za-z.-]+)?', version)
    if not match or int(match[2]) < 7:
        raise SetupError('selected CLI must be a stable Forge >=1.7.0,<2.0.0; select a compatible binary')
    contracts = json.loads(probe([str(binary), '--json', 'contracts'], root))
    capabilities = contracts.get('contracts') if isinstance(contracts, dict) else None
    if not isinstance(capabilities, dict) or any(
            type(capabilities.get(key)) is not int or capabilities[key] != 1
            for key in ('command_catalog', 'continuity')):
        raise SetupError('selected CLI requires command_catalog and continuity schema 1')
    return str(binary), match[1]


def no_symlink_ancestors(path):
    for part in reversed((path, *path.parents)):
        try:
            info = part.lstat()
        except FileNotFoundError:
            continue
        if stat.S_ISLNK(info.st_mode) or getattr(info, 'st_file_attributes', 0) & 0x400:
            raise SetupError('symlink/reparse-point ancestors are refused; select a direct project path')
        if part != path and not stat.S_ISDIR(info.st_mode):
            raise SetupError('installation ancestor is not a directory')


def project_target(root, host):
    if '..' in root.parts:
        raise SetupError('--root must not contain parent traversal')
    root = Path(os.path.abspath(root))
    target = root / ('.agents' if host == 'codex' else '.cursor') / 'skills' / SKILL
    no_symlink_ancestors(target)
    if not root.is_dir() or not os.path.lexists(root / '.git'):
        raise SetupError('--root must select an existing Git project root')
    no_symlink_ancestors(root / '.git')
    env = {key: value for key, value in os.environ.items() if not key.startswith('GIT_')}
    env['GIT_OPTIONAL_LOCKS'] = '0'
    observed = probe(['git', '-C', str(root), 'rev-parse', '--show-toplevel'], root, env)
    if Path(os.path.abspath(observed)) != root:
        raise SetupError('--root must be the selected Git top-level directory')
    if os.path.lexists(target):
        raise SetupError('existing skill target preserved; inspect it or select another project (no overwrite)')
    return root, target


def apply_files(root, target, files):
    # Descriptor-relative exclusive creation closes ancestor replacement races on Unix.
    # Fail closed on platforms without these primitives; Windows is experimental.
    if not hasattr(os, 'O_NOFOLLOW') or os.open not in os.supports_dir_fd:
        raise SetupError('safe directory-relative apply unavailable on this platform; Windows remains experimental')
    flags = os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW
    created = []
    with contextlib.ExitStack() as stack:
        def opened(name, parent=None):
            fd = os.open(name, flags, dir_fd=parent)
            stack.callback(os.close, fd)
            return fd

        def remember(parent, name, fd, directory):
            info = os.fstat(fd)
            created.append((parent, name, info.st_dev, info.st_ino, directory))

        def directory(parent, name, exclusive=False):
            made = False
            try:
                os.mkdir(name, 0o700, dir_fd=parent)
                made = True
            except FileExistsError:
                if exclusive:
                    raise SetupError('existing skill target preserved; no overwrite') from None
            fd = opened(name, parent)
            if made:
                remember(parent, name, fd, True)
            return fd

        try:
            fd = opened(root.anchor)
            for part in root.parts[1:]:
                fd = opened(part, fd)
            for part in target.relative_to(root).parts[:-1]:
                fd = directory(fd, part)
            destination = directory(fd, SKILL, exclusive=True)
            directories = {'': destination}
            for name, body in sorted(files.items()):
                parts = name.split('/')
                parent = destination
                for index, part in enumerate(parts[:-1], 1):
                    key = '/'.join(parts[:index])
                    if key not in directories:
                        directories[key] = directory(parent, part, exclusive=True)
                    parent = directories[key]
                file_fd = os.open(parts[-1], os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW,
                                  0o644, dir_fd=parent)
                remember(parent, parts[-1], file_fd, False)
                with os.fdopen(file_fd, 'wb') as output:
                    output.write(body)
            for parent, name, device, inode, is_dir in reversed(created):
                if is_dir:
                    info = os.stat(name, dir_fd=parent, follow_symlinks=False)
                    if (info.st_dev, info.st_ino) != (device, inode):
                        raise SetupError('installation directory changed during apply')
            # Recheck the named path before reporting success; fds never followed a link.
            no_symlink_ancestors(target)
            named, held = target.lstat(), os.fstat(destination)
            if (named.st_dev, named.st_ino) != (held.st_dev, held.st_ino):
                raise SetupError('installation target changed during apply')
        except BaseException:
            # Remove only entries we created, and only while their identity still matches.
            for parent, name, device, inode, is_dir in reversed(created):
                with contextlib.suppress(OSError):
                    info = os.stat(name, dir_fd=parent, follow_symlinks=False)
                    if (info.st_dev, info.st_ino) == (device, inode):
                        (os.rmdir if is_dir else os.unlink)(name, dir_fd=parent)
            raise


def install(archive, sha256, root, host, forge_binary, apply=False):
    files, digest = verify_archive(archive, sha256)
    root, target = project_target(root, host)
    binary, version = explicit_cli(forge_binary, root)
    if apply:
        apply_files(root, target, files)
    return {'schema_version': 1, 'status': 'installed' if apply else 'preview', 'applied': apply,
            'skill': SKILL, 'host': host, 'target': str(target), 'archive_sha256': digest,
            'forge_binary': binary, 'forge_version': version,
            'contracts': {'command_catalog': 1, 'continuity': 1},
            'files': [{'path': name, 'sha256': hashlib.sha256(body).hexdigest()}
                      for name, body in sorted(files.items())]}


def main():
    parser = argparse.ArgumentParser(description=__doc__, allow_abbrev=False)
    parser.add_argument('--archive', required=True, type=Path)
    parser.add_argument('--sha256', required=True)
    parser.add_argument('--root', required=True, type=Path)
    parser.add_argument('--host', required=True, choices=('codex', 'cursor'),
                        help='Skill loader: codex uses .agents/skills for Codex CLI and the Codex '
                             'extension in VS Code or Cursor; cursor uses .cursor/skills for the '
                             'native Cursor Agent, not the Codex extension. Choose one copy.')
    parser.add_argument('--forge-binary', required=True, type=Path)
    parser.add_argument('--apply', action='store_true', help='Create one new project skill installation')
    args = parser.parse_args()
    try:
        receipt = install(**vars(args))
    except (OSError, ValueError, zipfile.BadZipFile, RuntimeError) as error:
        message = str(error) if isinstance(error, SetupError) else 'invalid archive or setup input; verify paths, ZIP and CLI JSON'
        print(json.dumps({'schema_version': 1, 'status': 'blocked', 'error': message}), file=sys.stderr)
        return 2
    print(json.dumps(receipt, sort_keys=True))
    return 0


if __name__ == '__main__':
    sys.exit(main())
