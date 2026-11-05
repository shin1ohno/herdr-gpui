#!/usr/bin/env bash
# Run the existing native test executable in a private foreground UIKit app.
# No Cargo invocation here: this is called while Cargo owns its build lock.
set -euo pipefail
udid=${1:?simulator UDID required}
binary=${2:?test executable required}
shift 2
bundle=so.pen.herdr-gpui.clipboard-tests
work=$(mktemp -d "${TMPDIR:-/tmp}/herdr-ios-clipboard.XXXXXX")
trap 'rm -rf "$work"' EXIT
app=$work/ClipboardTests.app
mkdir -p "$app"
cp "$binary" "$app/ios_clipboard"
python3 - "$app/Info.plist" <<'PY'
import plistlib,sys
with open(sys.argv[1], 'wb') as f:
    plistlib.dump({
        'CFBundleIdentifier':'so.pen.herdr-gpui.clipboard-tests',
        'CFBundleExecutable':'ios_clipboard', 'CFBundleName':'Clipboard Tests',
        'CFBundlePackageType':'APPL', 'CFBundleVersion':'1',
        'CFBundleShortVersionString':'1.0', 'MinimumOSVersion':'17.0',
        'CFBundleSupportedPlatforms':['iPhoneSimulator'], 'UIDeviceFamily':[2],
        'UILaunchScreen':{},
        'UIApplicationSceneManifest':{'UIApplicationSupportsMultipleScenes':False},
    }, f)
PY
codesign --force --sign - "$app"
xcrun simctl install "$udid" "$app"
container=$(xcrun simctl get_app_container "$udid" "$bundle" data)
result=$container/Documents/clipboard-result.txt
mkdir -p "$(dirname "$result")"
rm -f "$result"
export SIMCTL_CHILD_HERDR_TEST_CLIPBOARD_RESULT=$result
xcrun simctl launch --console --terminate-running-process "$udid" "$bundle" "$@"
if [[ ! -f $result ]] || [[ $(cat "$result") != passed ]]; then
  echo "UIKit clipboard native tests failed or did not write their result: $result" >&2
  exit 1
fi
