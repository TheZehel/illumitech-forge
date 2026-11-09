"""Behavioral installation checks against real packaged ZIP bytes, without native inference."""
import hashlib
import io
import json
import os
from pathlib import Path
import shutil
import stat
import subprocess
import sys
import tarfile
import tempfile
import unittest
from unittest.mock import patch
import warnings
import zipfile

import install_agent_skill as installer
from package import package as binary_package
from package_agent_skill import ROOT, package


SAFE_APPLY = hasattr(os, 'O_NOFOLLOW') and os.open in os.supports_dir_fd


def snapshot(root):
    return {str(p.relative_to(root)): (p.lstat().st_mode,
            os.readlink(p) if p.is_symlink() else p.read_bytes() if p.is_file() else None)
            for p in root.rglob('*')}


@unittest.skipUnless(os.name == 'posix', 'Unix executable fixtures; Windows native pilot pending')
class Installation(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.base = Path(self.temp.name).resolve()
        artifacts = package(ROOT, self.base / 'artifacts')
        self.archive = self.base / 'artifacts' / artifacts['archives'][0]['file']
        self.digest = artifacts['archives'][0]['sha256']
        self.root = self.project('project')
        self.cli = self.base / 'selected-forge'
        self.fake_cli()

    def project(self, name):
        root = self.base / name
        root.mkdir()
        subprocess.run(['git', 'init', '-q', str(root)], check=True, capture_output=True)
        (root / 'unrelated.txt').write_bytes(b'preserve\n')
        return root

    def fake_cli(self, version='illumitech-forge 1.7.1', contracts=None, body=None):
        if contracts is None:
            contracts = {'contracts': {'command_catalog': 1, 'continuity': 1}}
        self.cli.write_text(f'#!{sys.executable}\nimport sys\n' + (body or
            f'assert sys.argv[1:] in (["--version"], ["--json", "contracts"])\n'
            f'print({version!r} if sys.argv[1:] == ["--version"] else {json.dumps(contracts)!r})\n'))
        self.cli.chmod(0o755)

    def run_install(self, **kwargs):
        args = dict(archive=self.archive, sha256=self.digest, root=self.root,
                    host='codex', forge_binary=self.cli)
        args.update(kwargs)
        return installer.install(**args)

    def rewrite(self, change, resign=True):
        with zipfile.ZipFile(self.archive) as source:
            entries = [(info, source.read(info)) for info in source.infolist()]
        entries = change(entries)
        if resign:
            entries = [(i, b) for i, b in entries if i.filename != 'SHA256SUMS']
            sums = ''.join(f'{hashlib.sha256(b).hexdigest()}  {i.filename}\n' for i, b in entries)
            entries.append((zipfile.ZipInfo('SHA256SUMS'), sums.encode()))
        output = self.base / 'changed.zip'
        with warnings.catch_warnings():
            warnings.simplefilter('ignore', UserWarning)
            with zipfile.ZipFile(output, 'w') as packed:
                for info, body in entries:
                    packed.writestr(info, body)
        return output, hashlib.sha256(output.read_bytes()).hexdigest()

    def assert_refused(self, **kwargs):
        root = kwargs.get('root', self.root)
        before = snapshot(root)
        with self.assertRaises((installer.SetupError, ValueError, OSError, zipfile.BadZipFile)):
            self.run_install(apply=True, **kwargs)
        self.assertEqual(snapshot(root), before)

    def test_preview_is_read_only_for_each_host(self):
        before = snapshot(self.root)
        for host, parent in [('codex', '.agents'), ('cursor', '.cursor')]:
            receipt = self.run_install(host=host)
            self.assertEqual(receipt['status'], 'preview')
            self.assertFalse(receipt['applied'])
            self.assertEqual(Path(receipt['target']), self.root / parent / 'skills/illumitech-forge')
            self.assertEqual(snapshot(self.root), before)

    @unittest.skipUnless(SAFE_APPLY, 'safe descriptor-relative apply unavailable')
    def test_apply_exact_bytes_preserves_other_skills_and_refuses_repeat(self):
        for host, parent in [('codex', '.agents'), ('cursor', '.cursor')]:
            root = self.project(host)
            other = root / parent / 'skills/other-skill/keep.txt'
            other.parent.mkdir(parents=True)
            other.write_bytes(b'other skill')
            before = snapshot(root)
            receipt = self.run_install(root=root, host=host, apply=True)
            target = Path(receipt['target'])
            with zipfile.ZipFile(self.archive) as source:
                for item in receipt['files']:
                    name = item['path']
                    expected = source.read(name if name == 'LICENSE' else 'illumitech-forge/' + name)
                    self.assertEqual((target / name).read_bytes(), expected)
            self.assertEqual(set(snapshot(target)), set(installer.RESOURCES) |
                             {'LICENSE', 'agents', 'references'})
            for name, value in before.items():
                self.assertEqual(snapshot(root)[name], value)
            installed = snapshot(root)
            with self.assertRaisesRegex(installer.SetupError, 'preserved'):
                self.run_install(root=root, host=host, apply=True)
            self.assertEqual(snapshot(root), installed)

    def test_missing_old_new_wrong_and_schema_incompatible_cli(self):
        for version in ['illumitech-forge 1.6.9', 'illumitech-forge 2.0.0',
                        'other-forge 1.7.1', 'forge 1.7.0-rc.1']:
            self.fake_cli(version=version)
            self.assert_refused()
        for contracts in [{}, [], {'contracts': {'command_catalog': 2, 'continuity': 1}},
                          {'contracts': {'command_catalog': 1, 'continuity': 2}},
                          {'contracts': {'command_catalog': True, 'continuity': 1}}]:
            self.fake_cli(contracts=contracts)
            self.assert_refused()
        self.fake_cli(body='print("not JSON")\n')
        self.assert_refused()
        self.fake_cli(body='print("forge 1.7.1" if sys.argv[1:] == ["--version"] else "not JSON")\n')
        self.assert_refused()
        self.assert_refused(forge_binary=self.base / 'missing')

    def test_only_selected_cli_is_probed_never_path_forge(self):
        trap = self.base / 'forge'
        trap.write_text(f'#!{sys.executable}\nraise RuntimeError("PATH trap")\n')
        trap.chmod(0o755)
        with patch.dict(os.environ, {'PATH': str(self.base) + os.pathsep + os.environ['PATH']}):
            self.assertEqual(self.run_install()['forge_version'], '1.7.1')
            self.assert_refused(forge_binary=Path('nonexistent-selected-forge'))

    def test_compatible_floor_and_short_name_with_additive_contract_fields(self):
        self.fake_cli(version='forge 1.7.0', contracts={
            'contracts': {'command_catalog': 1, 'continuity': 1, 'future_capability': 9},
            'future_output': 'ignored'})
        self.assertEqual(self.run_install()['forge_version'], '1.7.0')

    def test_archive_hash_and_corrupt_zip(self):
        self.assert_refused(sha256='0' * 64)
        self.assert_refused(sha256='invalid')
        corrupt = self.base / 'corrupt.zip'
        corrupt.write_bytes(b'not a ZIP')
        self.assert_refused(archive=corrupt, sha256=hashlib.sha256(corrupt.read_bytes()).hexdigest())
        plugin = next((self.base / 'artifacts').glob('*-plugin.zip'))
        self.assert_refused(archive=plugin, sha256=hashlib.sha256(plugin.read_bytes()).hexdigest())

    def test_unsafe_extra_duplicate_missing_and_wrong_identity_even_with_valid_hashes(self):
        def extra(name):
            return lambda entries: entries + [(zipfile.ZipInfo(name), b'bad')]
        changes = [extra('../outside'), extra('/absolute'), extra('C:/drive'),
                   extra('illumitech-forge/../outside'), extra('illumitech-forge\\outside'),
                   extra('illumitech-forge/extra'), extra('illumitech-forge/SKILL.md\x00suffix'),
                   lambda entries: entries + [entries[0]],
                   lambda entries: entries[1:],
                   lambda entries: [(i, b.replace(b'name: illumitech-forge', b'name: forge-workflow'))
                                    if i.filename.endswith('SKILL.md') else (i, b) for i, b in entries]]
        for change in changes:
            with self.subTest(change=change):
                archive, sha = self.rewrite(change)
                self.assert_refused(archive=archive, sha256=sha)

    def test_symlink_and_special_zip_entries(self):
        for mode in (stat.S_IFLNK | 0o777, stat.S_IFIFO | 0o644, stat.S_IFDIR | 0o755):
            def change(entries):
                entries[0][0].create_system = 3
                entries[0][0].external_attr = mode << 16
                return entries
            archive, sha = self.rewrite(change)
            self.assert_refused(archive=archive, sha256=sha)

    def test_inner_checksums_complete_unique_and_accurate(self):
        for mutate in (lambda b: b.splitlines(keepends=True)[0],
                       lambda b: b + b.splitlines(keepends=True)[0],
                       lambda b: b.replace(b'  LICENSE', b'  missing'),
                       lambda b: b'0' * 64 + b[64:]):
            archive, sha = self.rewrite(lambda entries: [
                (i, mutate(b) if i.filename == 'SHA256SUMS' else b) for i, b in entries], resign=False)
            self.assert_refused(archive=archive, sha256=sha)
        archive, sha = self.rewrite(lambda entries: [
            (i, b + b'changed' if i.filename.endswith('setup.md') else b) for i, b in entries], resign=False)
        self.assert_refused(archive=archive, sha256=sha)

    def test_bounds_on_archive_members_and_probes(self):
        with patch.object(installer, 'MAX_ARCHIVE', 10):
            self.assert_refused()
        with patch.object(installer, 'MAX_FILE', 10):
            self.assert_refused()
        with patch.object(installer, 'MAX_TOTAL', 10):
            self.assert_refused()
        with patch.object(installer, 'MAX_ENTRIES', 2):
            self.assert_refused()
        self.fake_cli(body='import time\ntime.sleep(5)\n')
        with patch.object(installer, 'PROBE_SECONDS', 0.1):
            self.assert_refused()
        self.fake_cli(body='print("x" * 20000)\n')
        with patch.object(installer, 'PROBE_LIMIT', 10000):
            self.assert_refused()

    def test_expanded_file_bound_and_crc_corruption_on_real_zip_bytes(self):
        archive, sha = self.rewrite(lambda entries: [
            (i, b'x' * (installer.MAX_FILE + 1) if i.filename.endswith('setup.md') else b)
            for i, b in entries])
        self.assert_refused(archive=archive, sha256=sha)
        # Change a stored payload without changing its CRC, then supply the correct outer hash.
        data = io.BytesIO()
        with zipfile.ZipFile(data, 'w', compression=zipfile.ZIP_STORED) as packed:
            with zipfile.ZipFile(self.archive) as source:
                for info in source.infolist():
                    packed.writestr(info.filename, source.read(info))
        corrupted = bytearray(data.getvalue())
        with zipfile.ZipFile(io.BytesIO(corrupted)) as packed:
            info = packed.getinfo('illumitech-forge/SKILL.md')
            offset = info.header_offset + 30 + len(info.filename.encode()) + len(info.extra)
        corrupted[offset] ^= 1
        archive = self.base / 'crc.zip'
        archive.write_bytes(corrupted)
        self.assert_refused(archive=archive, sha256=hashlib.sha256(corrupted).hexdigest())

    def test_existing_empty_file_directory_and_dangling_target_preserved(self):
        target = self.root / '.agents/skills/illumitech-forge'
        target.parent.mkdir(parents=True)
        for kind in ['file', 'empty', 'nonempty', 'dangling']:
            if kind == 'file':
                target.write_bytes(b'existing')
            elif kind == 'dangling':
                target.symlink_to(self.base / 'absent', target_is_directory=True)
            else:
                target.mkdir()
                if kind == 'nonempty':
                    (target / 'custom.txt').write_text('custom')
            self.assert_refused()
            if target.is_symlink() or target.is_file():
                target.unlink()
            else:
                shutil.rmtree(target)

    def test_symlink_root_and_every_destination_ancestor_preserved(self):
        direct = self.base / 'linked-project'
        direct.symlink_to(self.root, target_is_directory=True)
        self.assert_refused(root=direct)
        for host, parent in [('codex', '.agents'), ('cursor', '.cursor')]:
            for depth, part in enumerate([parent, parent + '/skills']):
                root = self.project(f'{host}-{depth}')
                outside = self.base / f'outside-{host}-{depth}'
                outside.mkdir()
                link = root / part
                link.parent.mkdir(parents=True, exist_ok=True)
                link.symlink_to(outside, target_is_directory=True)
                before = snapshot(outside)
                self.assert_refused(root=root, host=host)
                self.assertEqual(snapshot(outside), before)

    def test_non_root_and_missing_git_are_rejected(self):
        self.assert_refused(root=self.base / 'missing')
        not_git = self.base / 'not-git'
        not_git.mkdir()
        self.assert_refused(root=not_git)
        subdir = self.root / 'nested'
        subdir.mkdir()
        self.assert_refused(root=subdir)

    @unittest.skipUnless(SAFE_APPLY, 'safe descriptor-relative apply unavailable')
    def test_exclusive_creation_and_failure_cleanup_preserve_existing_tree(self):
        before = snapshot(self.root)
        original = os.open
        def fail(name, flags, *args, **kwargs):
            if name == 'setup.md':
                raise OSError('disposable disk failure')
            return original(name, flags, *args, **kwargs)
        # Patch after platform capability detection so it remains the real contract.
        with patch.object(installer.os, 'open', side_effect=fail) as opened:
            with patch.object(installer.os, 'supports_dir_fd', os.supports_dir_fd | {opened}):
                with self.assertRaises(OSError):
                    installer.apply_files(self.root, self.root / '.agents/skills/illumitech-forge',
                                          installer.verify_archive(self.archive, self.digest)[0])
        self.assertEqual(snapshot(self.root), before)

    @unittest.skipUnless(SAFE_APPLY, 'safe descriptor-relative apply unavailable')
    def test_racing_target_or_symlink_after_preview_is_not_followed_or_overwritten(self):
        files, _ = installer.verify_archive(self.archive, self.digest)
        target = Path(self.run_install()['target'])
        target.mkdir(parents=True)
        before = snapshot(self.root)
        with self.assertRaises(installer.SetupError):
            installer.apply_files(self.root, target, files)
        self.assertEqual(snapshot(self.root), before)
        shutil.rmtree(self.root / '.agents')
        outside = self.base / 'racing-outside'
        outside.mkdir()
        (self.root / '.agents').symlink_to(outside, target_is_directory=True)
        before = snapshot(self.root)
        with self.assertRaises(OSError):
            installer.apply_files(self.root, target, files)
        self.assertEqual(snapshot(self.root), before)
        self.assertEqual(snapshot(outside), {})

    @unittest.skipUnless(SAFE_APPLY, 'safe descriptor-relative apply unavailable')
    def test_shipped_script_works_standalone_from_binary_archive(self):
        # A fixture binary establishes packaging, not native host acceptance.
        binary = self.base / 'illumitech-forge'
        short = self.base / 'forge'
        binary.write_bytes(b'fixture')
        short.write_bytes(b'fixture')
        with context_output():
            archive = binary_package(ROOT, 'aarch64-apple-darwin', binary, self.base / 'binary', short)
        with tarfile.open(archive) as packed:
            body = packed.extractfile('scripts/install_agent_skill.py').read()
            self.assertEqual(body, (ROOT / 'scripts/install_agent_skill.py').read_bytes())
            sums = packed.extractfile('SHA256SUMS').read().decode()
            self.assertIn(hashlib.sha256(body).hexdigest() + '  scripts/install_agent_skill.py', sums)
        script = self.base / 'standalone.py'
        script.write_bytes(body)
        argv = [sys.executable, '-I', str(script), '--archive', str(self.archive),
                '--sha256', self.digest, '--root', str(self.root), '--host', 'codex',
                '--forge-binary', str(self.cli)]
        before = snapshot(self.root)
        preview = subprocess.run(argv, cwd=self.base, capture_output=True, text=True, check=True)
        self.assertEqual(json.loads(preview.stdout)['status'], 'preview')
        self.assertEqual(snapshot(self.root), before)
        applied = subprocess.run(argv + ['--apply'], cwd=self.base, capture_output=True, text=True, check=True)
        self.assertTrue(json.loads(applied.stdout)['applied'])
        refused = subprocess.run(argv + ['--apply'], cwd=self.base, capture_output=True, text=True)
        self.assertEqual(refused.returncode, 2)
        self.assertEqual(json.loads(refused.stderr)['status'], 'blocked')


def context_output():
    from contextlib import redirect_stdout
    return redirect_stdout(io.StringIO())


if __name__ == '__main__':
    unittest.main()
