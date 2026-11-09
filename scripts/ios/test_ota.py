"""No network, real signing, or device installation."""
from pathlib import Path
import plistlib
import tempfile
import unittest
import zipfile

from ota import generate
from distribution import BUNDLE


class OtaTests(unittest.TestCase):
    def test_preview_and_install_manifest(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            ipa = root / 'fixture.ipa'
            with zipfile.ZipFile(ipa, 'w') as archive:
                archive.writestr('Payload/Herdr.app/Info.plist', plistlib.dumps({
                    'CFBundleIdentifier': BUNDLE, 'CFBundleVersion': '12',
                    'CFBundleShortVersionString': '1.0', 'CFBundleSupportedPlatforms': ['iPhoneOS']}))
            for preview in (True, False):
                output = root / str(preview)
                generate(ipa, output, 'https://example.invalid/build12', preview)
                manifest = plistlib.loads((output / 'manifest.plist').read_bytes())
                self.assertEqual(manifest['items'][0]['assets'][0]['url'], 'https://example.invalid/build12/app.ipa')
                self.assertEqual(manifest['items'][0]['metadata']['bundle-version'], '12')
                self.assertEqual((output / 'app.ipa').read_bytes(), ipa.read_bytes())
                self.assertEqual('itms-services://' in (output / 'index.html').read_text(), not preview)
            with self.assertRaises(ValueError):
                generate(ipa, root / 'True', 'https://example.invalid')
            for url in ('http://localhost', 'https://user:password@example.invalid', 'https://example.invalid/?token=x'):
                with self.subTest(url=url), self.assertRaises(ValueError):
                    generate(ipa, root / 'bad', url)


if __name__ == '__main__':
    unittest.main()
