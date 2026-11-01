#!/usr/bin/env bash
# Check both iPadOS targets, including the real application's UIKit entry point.
set -euo pipefail

root=$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)
cd "$root"
for script in scripts/ios/*.sh; do bash -n "$script"; done
python3 -I -B -c 'import ast, sys; [ast.parse(open(p, encoding="utf-8").read(), p) for p in sys.argv[1:]]' scripts/ios/*.py
python3 -I -B -m unittest discover -s scripts/ios -p 'test_*.py'
source scripts/ios/lib.sh
ios_export_deployment_target

for target in aarch64-apple-ios aarch64-apple-ios-sim; do
  cargo clippy --locked --all-targets --target "$target" \
    -p herdr-uikit -p herdr-client -p herdr-credentials -p herdr-ssh \
    -p herdr-backend -p herdr-core-client -- -D warnings
  cargo clippy --locked -p herdr-gpui --no-default-features \
    --all-targets --target "$target" -- -D warnings
done
