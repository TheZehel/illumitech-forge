"""Candidate tests use disposable local source; no publisher or native agent calls."""
import hashlib
import json
import os
from pathlib import Path
import subprocess
import tarfile
import tempfile
import unittest
from candidate import seal, source_files
from package import write_archive


class CandidateContract(unittest.TestCase):
    def test_archives_are_deterministic_with_inventory_and_executable_modes(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            source = root / 'tool'
            source.write_bytes(b'stable bytes')
            source.chmod(0o755)
            for suffix in ('.tar.gz', '.zip'):
                first, second = root / ('first' + suffix), root / ('second' + suffix)
                write_archive(first, {'tool': source})
                os.utime(source, (10000000, 10000000))
                write_archive(second, {'tool': source})
                self.assertEqual(first.read_bytes(), second.read_bytes())
            with tarfile.open(root / 'first.tar.gz') as packed:
                self.assertEqual(packed.getmember('tool').mode, 0o755)
                expected = hashlib.sha256(source.read_bytes()).hexdigest()
                self.assertEqual(packed.extractfile('SHA256SUMS').read().decode(), expected + '  tool\n')

    def test_seal_binds_current_dirty_bytes_and_never_resurrects_ignored_state(self):
        with tempfile.TemporaryDirectory() as temp:
            base = Path(temp)
            root = base / 'project'
            root.mkdir()
            subprocess.run(['git', 'init', '-q', root], check=True)
            (root / 'Cargo.toml').write_text('[workspace.package]\nversion="1.0.0"\n')
            (root / '.gitignore').write_text('target/\n')
            (root / 'target').mkdir()
            (root / 'target/state.sqlite').write_bytes(b'private state')
            (root / 'source.rs').write_text('first')
            subprocess.run(['git', '-C', root, 'add', '.'], check=True)
            (root / 'source.rs').write_text('dirty')
            output = base / 'candidate'
            receipt = seal(root, output, [])
            self.assertEqual(receipt['publication'], 'not_performed')
            with tarfile.open(next(output.glob('*.tar.gz'))) as packed:
                self.assertEqual(packed.extractfile('source.rs').read(), b'dirty')
                self.assertNotIn('target/state.sqlite', packed.getnames())
            record = json.loads((output / 'candidate.json').read_text())
            self.assertTrue(any(i['path'] == 'source.rs' for i in record['source_inventory']))
            with self.assertRaises(ValueError):
                seal(root, output, [])
            (root / 'credential.txt').write_text('ghp_' + 'x' * 36)
            with self.assertRaises(ValueError):
                source_files(root)
            (root / 'credential.txt').unlink()
            (root / 'outside-link').symlink_to(base / 'outside')
            with self.assertRaises(ValueError):
                source_files(root)


if __name__ == '__main__':
    unittest.main()
