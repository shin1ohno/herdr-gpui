"""Generate an offline OTA preview; publishing and signing are separate opt-in steps."""
import argparse
import html
from pathlib import Path
import plistlib
import shutil
from urllib.parse import quote, urlsplit

from distribution import ipa_metadata


def generate(ipa, output, base_url, preview=False):
    url = urlsplit(base_url)
    if url.scheme != 'https' or not url.netloc or url.query or url.fragment or url.username or url.password:
        raise ValueError('base URL must be HTTPS without credentials, query or fragment')
    info = ipa_metadata(ipa)
    if output.exists():
        raise ValueError('choose a fresh output directory')
    base = base_url.rstrip('/')
    manifest = {'items': [{'assets': [{'kind': 'software-package', 'url': base + '/app.ipa'}],
        'metadata': {'bundle-identifier': info['CFBundleIdentifier'],
                     'bundle-version': info['CFBundleVersion'], 'kind': 'software',
                     'title': 'Sheprd'}}]}
    output.mkdir(parents=True)
    shutil.copyfile(ipa, output / 'app.ipa')
    (output / 'manifest.plist').write_bytes(plistlib.dumps(manifest))
    link = 'itms-services://?action=download-manifest&url=' + quote(base + '/manifest.plist', safe='')
    body = ('<p>Offline preview: installation is disabled.</p>' if preview else
            '<a href="' + html.escape(link, quote=True) + '">Install Sheprd</a>')
    (output / 'index.html').write_text('<!doctype html><meta charset="utf-8"><title>Sheprd</title>' + body)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--ipa', required=True, type=Path)
    parser.add_argument('--output', required=True, type=Path)
    parser.add_argument('--base-url', default='https://example.invalid/sheprd')
    parser.add_argument('--enable-install-link', action='store_true')
    args = parser.parse_args()
    try:
        generate(args.ipa, args.output, args.base_url, preview=not args.enable_install_link)
    except (ValueError, OSError) as error:
        parser.exit(1, str(error) + '\n')


if __name__ == '__main__':
    main()
