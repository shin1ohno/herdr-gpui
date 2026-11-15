#!/usr/bin/env bash
# Rule R10 on the toolchain CI uses. rustdoc checks a `compile_fail` doctest's
# error code only on nightly, so on stable `compile_fail,E0451` passes for any
# compile error. This compiles the forgery snippet with that toolchain's rustc
# against the built crate and requires error E0451 (private field), after a
# control snippet with the same paths has compiled. It checks the struct
# literal only; a public constructor, `Default`, or `Clone` is for review.
set -euo pipefail

root=$(git rev-parse --show-toplevel)
cd "$root"
snippets="$root/crates/herdr-ssh/scripts/receipt-forgery"
out=$(mktemp -d)
trap 'rm -rf "$out"' EXIT

rlib=$(cargo build --locked -p herdr-ssh --lib --message-format=json | python3 -c '
import json, sys
for line in sys.stdin:
    message = json.loads(line)
    target = message.get("target") or {}
    if message.get("reason") == "compiler-artifact" and target.get("name") == "herdr_ssh":
        for name in message["filenames"]:
            if name.endswith(".rlib"):
                print(name)
')
if [[ -z $rlib ]]; then
  echo "check-receipt-forgery: no herdr_ssh rlib was built" >&2
  exit 1
fi
# Cargo reports the primary crate's copy in target/<profile>; the crates it
# links against stay in its deps directory.
built="$(dirname "$rlib")"
deps="$built/deps"
[[ -d $deps ]] || deps="$built"

compile() {
  rustc --edition 2024 --crate-type lib --crate-name "$1" \
    -L "dependency=$deps" --extern "herdr_ssh=$rlib" \
    -o "$out/lib$1.rlib" "$snippets/$1.rs"
}

rustc --version
compile control
echo "control.rs compiles against $rlib"
if compile forge 2>"$out/forge.txt"; then
  echo "check-receipt-forgery: forge.rs compiled; a TeardownReceipt can be built outside herdr-ssh" >&2
  exit 1
fi
if ! grep -q '^error\[E0451\]' "$out/forge.txt"; then
  cat "$out/forge.txt" >&2
  echo "check-receipt-forgery: forge.rs failed, but not with E0451 (private field)" >&2
  exit 1
fi
grep '^error' "$out/forge.txt"
echo "R10 holds: forge.rs fails with E0451 on $(rustc --version)"
