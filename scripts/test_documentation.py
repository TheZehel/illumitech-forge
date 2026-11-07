import json
import shutil
import tempfile
import unittest
from pathlib import Path
from documentation import ROOT, TARGETS, render
class Documentation(unittest.TestCase):
    def test_every_crate_readme_tracks_release_and_rejects_stale_installation(self):
        with tempfile.TemporaryDirectory() as d:
            root = Path(d)
            shutil.copytree(ROOT / 'release', root / 'release')
            published = json.loads((root / 'release/current.json').read_text())['published']
            for name in TARGETS:
                (root / name).parent.mkdir(parents=True, exist_ok=True)
            (root / f'docs/release-v{published}.md').write_text('Published record')
            render(root, True)
            for crate in ('cli', 'domain', 'adapters', 'catalog'):
                with self.subTest(crate=crate):
                    readme = root / f'crates/forge-{crate}/README.md'
                    self.assertTrue(readme.read_text().splitlines()[0].endswith(published))
                    readme.write_text('# Forge 1.4.1\ncargo install illumitech-forge-cli --version 1.4.1')
                    with self.assertRaisesRegex(ValueError, 'stale.*crates/forge-' + crate):
                        render(root)
                    render(root, True)
            self.assertIn(f'--version {published} --locked',
                          (root / 'crates/forge-cli/README.md').read_text())

    def test_current_installation_and_historical_versions_are_distinct(self):
        with tempfile.TemporaryDirectory() as d:
            root=Path(d)
            shutil.copytree(ROOT/'release',root/'release')
            (root/'docs').mkdir()
            published=json.loads((root/'release/current.json').read_text())['published']
            (root/f'docs/release-v{published}.md').write_text('Published record')
            (root/'docs/release-v1.2.0.md').write_text('Historical install --version 1.2.0')
            for name in TARGETS: (root/name).parent.mkdir(parents=True,exist_ok=True)
            render(root,True);self.assertEqual(render(root)['status'],'passed')
            (root/'docs/quickstart.md').write_text('cargo install --version 1.2.0')
            with self.assertRaisesRegex(ValueError,'stale'):render(root)
            render(root,True)
            p=root/'release/doc-templates/docs__quickstart.md.in';p.write_text(p.read_text()+'\ncargo install --version 1.2.0\n')
            with self.assertRaisesRegex(ValueError,'incoherent'):render(root)
if __name__=='__main__':unittest.main()
