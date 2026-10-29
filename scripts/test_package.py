"""Packaging contract only; a fixture EXE does not prove Windows execution."""
import hashlib
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
                     'scripts/install.ps1', 'scripts/uninstall.ps1', 'examples/windows/workflow.ps1',
                     'docs/windows-portability.md']
            for name in files:
                path = root / name
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_text('fixture: ' + name)
            binary = root / 'target/x86_64-pc-windows-msvc/release/illumitech-forge.exe'
            binary.parent.mkdir(parents=True)
            binary.write_bytes(b'fixture, not a Windows executable')
            subprocess.run([sys.executable, root / 'scripts/package.py', 'x86_64-pc-windows-msvc'], check=True, capture_output=True)
            archive, = (root / 'dist').glob('*.zip')
            self.assertIn('windows-candidate', archive.name)
            expected = archive.with_name(archive.name + '.sha256').read_text().split()[0]
            self.assertEqual(expected, hashlib.sha256(archive.read_bytes()).hexdigest())
            with zipfile.ZipFile(archive) as zip:
                self.assertEqual(set(zip.namelist()), set(files + ['illumitech-forge.exe', 'SHA256SUMS']))
                for line in zip.read('SHA256SUMS').decode().splitlines():
                    digest, name = line.split('  ', 1)
                    self.assertEqual(digest, hashlib.sha256(zip.read(name)).hexdigest())


if __name__ == '__main__':
    unittest.main()
