import json
import tempfile
import unittest
import zipfile
from pathlib import Path
from theme_pack import SKILL, load, pack


class PackageTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.source = self.root / 'source'
        self.source.mkdir()
        self.skin = json.loads((SKILL / 'assets/light/skin.json').read_text())
        self.save()

    def save(self):
        (self.source / 'skin.json').write_text(json.dumps(self.skin))

    def test_both_starters(self):
        for mode in ('light', 'dark'):
            load(SKILL / 'assets' / mode)

    def test_resources_round_trip_and_deterministic_zip(self):
        self.skin['resources']['background'] = 'assets/bg.png'
        self.save()
        (self.source / 'assets').mkdir()
        (self.source / 'assets/bg.png').write_bytes(b'image-fixture')
        (self.source / '.DS_Store').write_bytes(b'not-packaged')
        one, two = self.root / 'one.zip', self.root / 'two.zip'
        pack(self.source, one)
        pack(self.source, two)
        self.assertEqual(one.read_bytes(), two.read_bytes())
        data, files = load(one)
        self.assertEqual(data, self.skin)
        self.assertEqual(set(files), {'skin.json', 'assets/bg.png'})
        with self.assertRaises(ValueError):
            pack(self.source, one)

    def test_missing_and_wrong_resources(self):
        for resource in ['assets/missing.png', '../outside.png', '/tmp/bg.png', 'assets/code.js', 'assets/font.woff2']:
            self.skin['resources']['background'] = resource
            self.save()
            with self.assertRaises(ValueError):
                load(self.source)

    def test_no_local_resources_in_standalone_json(self):
        self.skin['resources']['font'] = 'font.woff2'
        self.save()
        with self.assertRaises(ValueError):
            load(self.source / 'skin.json')

    def test_injection_and_cycles(self):
        for value in ['red;}body{display:none', 'URL (https://bad)', 'red !IMPORTANT', 'var(--cycle)']:
            self.skin['colors']['--cycle'] = value
            self.save()
            with self.assertRaises(ValueError):
                load(self.source)

    def test_rejects_resource_symlink(self):
        outside = self.root / 'outside.png'
        outside.write_bytes(b'image')
        (self.source / 'bg.png').symlink_to(outside)
        self.skin['resources']['background'] = 'bg.png'
        self.save()
        with self.assertRaises(ValueError):
            load(self.source)

    def test_zip_traversal_and_outer_directory(self):
        for name in ['../skin.json', 'outer/skin.json']:
            path = self.root / 'bad.zip'
            with zipfile.ZipFile(path, 'w') as archive:
                archive.writestr(name, json.dumps(self.skin))
            with self.assertRaises(ValueError):
                load(path)

    def test_duplicate_manifest_keys(self):
        path = self.source / 'skin.json'
        path.write_text('{"id":"one","id":"two"}')
        with self.assertRaises(ValueError):
            load(path)


if __name__ == '__main__':
    unittest.main()
