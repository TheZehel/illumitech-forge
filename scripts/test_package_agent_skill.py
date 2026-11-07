import hashlib
import json
import tempfile
import unittest
import zipfile
from pathlib import Path
from package_agent_skill import ROOT, package

class AgentPackageTests(unittest.TestCase):
    def test_reproducible_complete_and_preserves_existing_outputs(self):
        with tempfile.TemporaryDirectory() as directory:
            base=Path(directory)
            first=package(ROOT,base/'one')
            second=package(ROOT,base/'two')
            self.assertEqual(first,second)
            for artifact in first['archives']:
                a=base/'one'/artifact['file']
                self.assertEqual(a.read_bytes(),(base/'two'/artifact['file']).read_bytes())
                self.assertEqual(hashlib.sha256(a.read_bytes()).hexdigest(),artifact['sha256'])
                with zipfile.ZipFile(a) as z:
                    sums=z.read('SHA256SUMS').decode().splitlines()
                    for line in sums:
                        digest,name=line.split('  ',1)
                        self.assertEqual(hashlib.sha256(z.read(name)).hexdigest(),digest)
                    if 'plugin' in a.name:
                        manifest=json.loads(z.read('plugin.json'))
                        self.assertEqual(manifest['version'],first['version'])
                        self.assertTrue(manifest['extensions']['com.openai']['requires_local_executor'])
                        self.assertIn('skills/illumitech-forge/references/feature-request.json',z.namelist())
                    self.assertFalse(any('core/' in name or name.endswith('.exe') or name.startswith('/') or '..' in Path(name).parts for name in z.namelist()))
            before=(base/'one'/'SHA256SUMS').read_bytes()
            with self.assertRaises(ValueError): package(ROOT,base/'one')
            self.assertEqual(before,(base/'one'/'SHA256SUMS').read_bytes())

if __name__=='__main__': unittest.main()
