import json
import io
from pathlib import Path
import subprocess
import tarfile
import tempfile
from unittest.mock import patch
import unittest
from distribution import ORDER, validate_files, validate_graph, verify_readme
from public_readiness import scan, forbidden_path, audit


class DistributionContract(unittest.TestCase):
    def packages(self):
        packages = {name: dict(name=name, version='0.3.0', description='fixture',
                              repository='https://example.invalid/repo', readme='README.md',
                              license='MIT', dependencies=[]) for name in ORDER}
        packages[ORDER[-1]]['dependencies'] = [dict(name=ORDER[0], path='../domain', req='=0.3.0')]
        return packages

    def test_order_and_inventory(self):
        packages = self.packages()
        validate_graph(packages, ORDER)
        with self.assertRaises(ValueError):
            validate_graph(packages, tuple(reversed(ORDER)))
        del packages[ORDER[1]]
        with self.assertRaises(ValueError):
            validate_graph(packages, ORDER)

    def test_unversioned_or_mismatched_dependency(self):
        for req in ['*', '=0.2.0', '^0.3.0']:
            packages = self.packages()
            packages[ORDER[-1]]['dependencies'][0]['req'] = req
            with self.assertRaises(ValueError):
                validate_graph(packages, ORDER)

    def test_package_allowlist(self):
        files = ['Cargo.toml', 'README.md', 'LICENSE', 'THIRD_PARTY_NOTICES', 'dependencies.json', 'src/lib.rs']
        validate_files(files)
        validate_files([entry.replace('\\', '/') for entry in files[:-1] + ['src\\lib.rs', 'THIRD_PARTY_LICENSES\\license.txt']])
        for bad in ['.env', 'src/.env.production', 'src/../secret', '/absolute', 'state.sqlite', 'docs/internal.md',
                    'src\\..\\secret', 'C:/secret', 'C:secret', '//server/share/secret']:
            with self.assertRaises(ValueError):
                validate_files(files + [bad])
        with self.assertRaises(ValueError):
            validate_files(['src/lib.rs'])

    def test_archive_readme_must_match_current_generated_source(self):
        current = b'# Illumitech Forge CLI 1.7.1\ncurrent installation\n'
        for body in (current, b'# Illumitech Forge CLI 1.4.1\nobsolete installation\n'):
            with self.subTest(body=body):
                archive = io.BytesIO()
                stem = 'illumitech-forge-cli-1.7.1'
                with tarfile.open(fileobj=archive, mode='w') as packed:
                    member = tarfile.TarInfo(stem + '/README.md')
                    member.size = len(body)
                    packed.addfile(member, io.BytesIO(body))
                archive.seek(0)
                with tarfile.open(fileobj=archive) as packed:
                    if body == current:
                        verify_readme(packed, stem, current)
                    else:
                        with self.assertRaisesRegex(ValueError, 'packaged README differs'):
                            verify_readme(packed, stem, current)

    def test_standalone_ui_allowlist_stays_narrow(self):
        files = ['Cargo.toml', 'README.md', 'LICENSE', 'THIRD_PARTY_NOTICES', 'dependencies.json',
                 'build.rs', 'ui-dist/index.html', 'ui-dist/logo.webp', 'ui-dist/THIRD_PARTY_LICENSES.txt',
                 'ui-dist/assets/index-abc123.js', 'ui-dist/assets/index-abc123.css']
        validate_files(files, ui=True)
        with self.assertRaises(ValueError):
            validate_files(files)
        for bad in ['ui-dist/.env', 'ui-dist/assets/private.js.map', 'ui-dist/install.sh', 'ui-dist/assets/../private.js']:
            with self.assertRaises(ValueError):
                validate_files(files + [bad], ui=True)
        with self.assertRaises(ValueError):
            validate_files([p for p in files if p != 'ui-dist/index.html'], ui=True)

    def test_redacted_audit_detection(self):
        self.assertIn('credential_token', scan(b'ghp_' + b'x' * 36))
        self.assertIn('personal_path', scan(b'/Users/' + b'fixture/private/'))
        self.assertEqual(scan(b'ordinary public source'), [])
        self.assertTrue(forbidden_path('config/.env.production'))
        self.assertTrue(forbidden_path('state/queue.sqlite'))
        self.assertFalse(forbidden_path('src/main.rs'))

    def test_deleted_secret_remains_a_history_blocker(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            def git(*args):
                subprocess.run(['git', *args], cwd=root, check=True, capture_output=True)
            git('init', '-q')
            secret = b'ghp_' + b'x' * 36
            (root / 'removed.txt').write_bytes(secret)
            git('add', '.')
            git('-c', 'user.name=Fixture', '-c', 'user.email=fixture@example.invalid',
                'commit', '-qm', 'fixture')
            (root / 'removed.txt').unlink()
            git('add', '-u')
            git('-c', 'user.name=Fixture', '-c', 'user.email=fixture@example.invalid',
                'commit', '-qm', 'remove fixture')
            (root / 'docs').mkdir()
            (root / 'docs/dependencies.json').write_text(json.dumps({'dependencies': [], 'missing_notices': []}))
            original = subprocess.check_output
            def output(args, **kwargs):
                if args[0] == 'cargo':
                    return b'{"packages": []}'
                return original(args, **kwargs)
            with patch('public_readiness.subprocess.check_output', side_effect=output):
                report = audit(root)
            self.assertEqual(report['status'], 'blocked')
            self.assertTrue(any(f['scope'] == 'history' and f['rule'] == 'credential_token' for f in report['findings']))
            self.assertFalse(any(f['scope'] == 'tree' for f in report['findings']))
            self.assertNotIn(secret.decode(), json.dumps(report))


if __name__ == '__main__':
    unittest.main()
