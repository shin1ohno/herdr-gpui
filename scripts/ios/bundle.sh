#!/usr/bin/env bash
# Build a Cargo example or binary for iPadOS and assemble it into an .app with
# scripts/ios/Info.plist, linked for that template's MinimumOSVersion. A
# simulator bundle is ad-hoc signed, which is all `xcrun simctl install`
# needs; a device bundle is left unsigned.
#
# The bundle's absolute path is the last line on stdout and everything else
# goes to stderr, so `app=$(scripts/ios/bundle.sh --example NAME)` works.
set -euo pipefail

usage() {
  cat <<'EOF'
Usage: scripts/ios/bundle.sh [options] (--example NAME | --bin NAME)

Options:
  --package PKG    Cargo package with the example or binary (default: herdr-uikit)
  --target TRIPLE  aarch64-apple-ios-sim, the simulator on Apple silicon
                   (default), or aarch64-apple-ios, a device
  --release        Build with the release profile (default: dev)
  --no-default-features
                   Disable the package's default Cargo features
  --features LIST  Enable the comma-separated Cargo features
  --bundle-id ID   CFBundleIdentifier (default: so.pen.herdr-gpui, with
                   .example.NAME appended for an example)
  -h, --help       Show this help

The bundle is written beside the built executable, as NAME.app.

For aarch64-apple-ios the bundle is built and assembled but not signed:
installing on an iPad needs an Apple development team and a provisioning
profile, which this script does not handle yet. It exits with status 3.
EOF
}

package=herdr-uikit
target=aarch64-apple-ios-sim
profile_args=()
feature_args=()
bundle_id=
kind=
name=

while (($# > 0)); do
  case "$1" in
    --example | --bin)
      if [[ -n $kind ]]; then
        echo "error: give one --example or --bin" >&2
        exit 2
      fi
      kind=${1#--}
      name=${2:?"$1 needs a name"}
      shift 2
      ;;
    --package)
      package=${2:?"--package needs a name"}
      shift 2
      ;;
    --target)
      target=${2:?"--target needs a triple"}
      shift 2
      ;;
    --release)
      profile_args=(--release)
      shift
      ;;
    --no-default-features)
      feature_args+=(--no-default-features)
      shift
      ;;
    --features)
      feature_args+=(--features "${2:?"--features needs a feature list"}")
      shift 2
      ;;
    --bundle-id)
      bundle_id=${2:?"--bundle-id needs an identifier"}
      shift 2
      ;;
    -h | --help)
      usage
      exit 0
      ;;
    *)
      echo "error: unknown argument: $1" >&2
      usage >&2
      exit 2
      ;;
  esac
done

if [[ -z $kind ]]; then
  usage >&2
  exit 2
fi

case "$target" in
  aarch64-apple-ios-sim) platform=iPhoneSimulator ;;
  aarch64-apple-ios) platform=iPhoneOS ;;
  *)
    echo "error: unsupported target $target (aarch64-apple-ios-sim or aarch64-apple-ios)" >&2
    exit 2
    ;;
esac

if [[ -z $bundle_id ]]; then
  bundle_id=so.pen.herdr-gpui
  # Bundle identifiers allow letters, digits, hyphens, and periods only.
  if [[ $kind == example ]]; then bundle_id+=.example.${name//_/-}; fi
fi
if [[ ! $bundle_id =~ ^[A-Za-z0-9][A-Za-z0-9.-]*$ ]]; then
  echo "error: '$bundle_id' is not a valid bundle identifier" >&2
  exit 2
fi

root=$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)
cd "$root"
# shellcheck source=scripts/ios/lib.sh
source scripts/ios/lib.sh
ios_export_deployment_target

if command -v rustup >/dev/null && ! rustup target list --installed | grep -qx "$target"; then
  echo "error: the Rust target $target is not installed: rustup target add $target" >&2
  exit 1
fi

# Cargo reports the built executable's path in its JSON messages, which
# follows CARGO_TARGET_DIR and profile settings this script need not know.
cargo_subcommand=build
link_args=()
if [[ $platform == iPhoneSimulator ]]; then
  # Simulator Security.framework reads this Mach-O section. A codesign-only
  # entitlement cannot grant Keychain access to an ad-hoc simulator binary.
  # Link only the final executable so dependencies retain their build cache.
  entitlements_dir=$(mktemp -d /tmp/herdr-ios-entitlements.XXXXXX)
  trap 'rm -rf -- "$entitlements_dir"' EXIT
  entitlements=$entitlements_dir/simulator.plist
  cat >"$entitlements" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
<key>application-identifier</key><string>HERDRSIM.$bundle_id</string>
<key>keychain-access-groups</key><array><string>HERDRSIM.$bundle_id</string></array>
</dict></plist>
EOF
  cargo_subcommand=rustc
  link_args=(-- -C "link-arg=-Wl,-sectcreate,__TEXT,__entitlements,$entitlements")
fi
messages=$(cargo "$cargo_subcommand" --locked --package "$package" --target "$target" \
  "--$kind" "$name" ${profile_args[@]+"${profile_args[@]}"} \
  ${feature_args[@]+"${feature_args[@]}"} --message-format=json-render-diagnostics \
  ${link_args[@]+"${link_args[@]}"})
artifact=$(python3 -I -c '
import json, sys

kind, name = sys.argv[1], sys.argv[2]
found = None
for line in sys.stdin:
    if not line.startswith("{"):
        continue
    message = json.loads(line)
    if message.get("reason") != "compiler-artifact" or not message.get("executable"):
        continue
    target = message["target"]
    if target["name"] == name and kind in target["kind"]:
        # A package ID ends in "#VERSION" or "#NAME@VERSION".
        version = message["package_id"].split("#", 1)[-1].rsplit("@", 1)[-1]
        found = (message["executable"], version)
if found is None:
    sys.exit(1)
print(*found, sep="\n")
' "$kind" "$name" <<<"$messages") || {
  echo "error: cargo built no $kind named $name in $package" >&2
  exit 1
}
executable=${artifact%$'\n'*}
version=${artifact##*$'\n'}

app="${executable%/*}/$name.app"
# The whole bundle is this script's own output, rebuilt from scratch so a
# renamed executable leaves nothing stale behind.
rm -rf "$app"
mkdir -p "$app"
cp "$executable" "$app/$name"

plist="$app/Info.plist"
cp scripts/ios/Info.plist "$plist"
plutil -replace CFBundleExecutable -string "$name" "$plist"
plutil -replace CFBundleIdentifier -string "$bundle_id" "$plist"
plutil -replace CFBundleShortVersionString -string "$version" "$plist"
plutil -replace CFBundleVersion -string "$version" "$plist"
plutil -replace CFBundleSupportedPlatforms -json "[\"$platform\"]" "$plist"
plutil -lint "$plist" >&2
if grep -q '\$(' "$plist"; then
  echo "error: $plist still has an unsubstituted \$(...) value" >&2
  exit 1
fi

if [[ $platform == iPhoneOS ]]; then
  echo "Assembled an unsigned device bundle: $app" >&2
  echo "error: installing on an iPad needs an Apple development team and a" \
    "provisioning profile; device signing is not supported by this script yet" >&2
  printf '%s\n' "$app"
  exit 3
fi

# Signing the bundle, not just the linker-signed executable, binds Info.plist
# to the signature, as the simulator's installer expects.
codesign --force --sign - --timestamp=none "$app" >&2
codesign --verify --strict "$app" >&2
echo "Bundled $bundle_id ($version) for the simulator" >&2
printf '%s\n' "$app"
