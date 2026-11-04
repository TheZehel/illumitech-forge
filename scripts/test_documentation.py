import json
import shutil
import tempfile
import unittest
from pathlib import Path
from documentation import ROOT, TARGETS, render
class Documentation(unittest.TestCase):
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
