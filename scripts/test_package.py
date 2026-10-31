"""Packaging contract only; a fixture EXE does not prove Windows execution."""
import hashlib
import os
import shutil
import subprocess
import sys
import tempfile
import unittest
import zipfile
from pathlib import Path


class WindowsPackage(unittest.TestCase):
    def test_zip_inventory_and_checksums(self):
        source = Path(__file__).resolve().parent
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            (root / 'scripts').mkdir()
            shutil.copyfile(source / 'package.py', root / 'scripts/package.py')
            (root / 'Cargo.toml').write_text('[workspace.package]\nversion="0.2.0"\n')
            files = ['LICENSE', 'THIRD_PARTY_NOTICES', 'THIRD_PARTY_LICENSES/dependency/LICENSE',
                     'docs/dependencies.json', 'docs/quickstart.md', 'docs/operational-parity.md',
                     'docs/contracts.md', 'scripts/demo.py', 'scripts/agent-fixtures.py',
                     'scripts/install.ps1', 'scripts/uninstall.ps1','scripts/windows-native-pilot.ps1', 'examples/windows/workflow.ps1',
                     'docs/windows-portability.md']
            for name in files:
                path = root / name
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_text('fixture: ' + name)
            binary = root / 'target/x86_64-pc-windows-msvc/release/illumitech-forge.exe'
            binary.parent.mkdir(parents=True)
            binary.write_bytes(b'fixture, not a Windows executable')
            binary.with_name('forge.exe').write_bytes(b'native forge fixture, not a Windows executable')
            subprocess.run([sys.executable, root / 'scripts/package.py', 'x86_64-pc-windows-msvc'], check=True, capture_output=True)
            archive, = (root / 'dist').glob('*.zip')
            self.assertIn('windows-candidate', archive.name)
            expected = archive.with_name(archive.name + '.sha256').read_text().split()[0]
            self.assertEqual(expected, hashlib.sha256(archive.read_bytes()).hexdigest())
            with zipfile.ZipFile(archive) as zip:
                self.assertEqual(set(zip.namelist()), set(files + ['illumitech-forge.exe', 'forge.exe', 'SHA256SUMS']))
                for line in zip.read('SHA256SUMS').decode().splitlines():
                    digest, name = line.split('  ', 1)
                    self.assertEqual(digest, hashlib.sha256(zip.read(name)).hexdigest())


class NativePackage(unittest.TestCase):
    @unittest.skipUnless(os.name == "posix", "Unix executable permission contract")
    def test_both_unix_executables_are_required_and_executable_in_archive(self):
        import tarfile
        from package import package
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            (root / 'Cargo.toml').write_text('[workspace.package]\nversion="1.0.0"\n')
            binary = root / 'artifacts/illumitech-forge'
            binary.parent.mkdir()
            binary.write_bytes(b'native long-name fixture')
            binary.chmod(0o755)
            with self.assertRaisesRegex(ValueError, 'missing native forge executable'):
                package(root, 'aarch64-apple-darwin', binary, root / 'missing-output')
            self.assertFalse((root / 'missing-output').exists())
            short = binary.with_name('forge')
            short.write_bytes(b'native short-name fixture')
            short.chmod(0o755)
            archive = package(root, 'aarch64-apple-darwin', binary, root / 'output')
            with tarfile.open(archive) as tar:
                for name in ['forge', 'illumitech-forge']:
                    self.assertEqual(tar.getmember(name).mode, 0o755)
                for row in tar.extractfile('SHA256SUMS').read().decode().splitlines():
                    digest, name = row.split('  ', 1)
                    self.assertEqual(digest, hashlib.sha256(tar.extractfile(name).read()).hexdigest())


if __name__ == '__main__':
    unittest.main()
