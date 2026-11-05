# Historical keyboard patch verification

These historical results apply only to the earlier `c3eb6e7` fix. They are
not acceptance evidence for the current integration; see [README.md](README.md)
for the current dependency and verification scope.

The earlier patch targets UIKit revision `00a8dba10b49f55332f6bc875f20ad8071f2d424`.
The client checkout was `b7a0c56f92b2689c7c2d7b20837ada3471a49873` on
`feat/ipados-foundation`. Initial validation used an untracked Cargo override.
At that verification point, the production dependency pinned the published fix at
[`c3eb6e736eb7e268d8f2b2806f1c7fb3ad3894e6`](https://github.com/shin1ohno/herdr-uikit/commit/c3eb6e736eb7e268d8f2b2806f1c7fb3ad3894e6).

## Regression tests

Tests failed before each corresponding fix:

- Two input-source routing tests exposed Control-Space interception by GPUI.
- A terminal input-handler test lost the marked text on UIKit unmarking.
- A document test moved an emoji selection from UTF-16 `1..3` to `5..5`
  while committing marked text.
- Two control-insertion tests exposed lost Return confirmation and Escape
  leakage. CR and LF, with and without an already-forwarded physical key,
  now commit once without submitting a newline. Escape cancels without sending
  text or a terminal Escape. Subsequent ordinary controls retain dispatch and
  duplicate-suppression behavior.

Final checks passed:

- `just format` and `git diff --check`.
- `just ci`, including default and all-feature workspace checks. The UI suites
  reported 5,429 passed / 4 ignored and 5,474 passed / 5 ignored, respectively.
- `just test-ios /path/to/patched/herdr-uikit/Cargo.toml`: 328 UIKit tests and
  75 Python tooling tests passed.
- `just lint-ios`: UIKit, client, SSH, credentials, backend, core client and
  application checks for both device and simulator targets passed.
- UIKit `cargo clippy --locked --all-targets --all-features` passed separately
  for `aarch64-apple-ios` and `aarch64-apple-ios-sim`.
- `scripts/ios/bundle.sh --package herdr-gpui --bin herdr-gpui --no-default-features`
  built the final application, which was installed and exercised again.
- The exported patch passed `git apply --check` against a clean archive of its
  base revision. Diagnostic instrumentation is excluded.

These runs used macOS arm64, Rust 1.96.1, Xcode 27.2, and debug builds with debug
symbols and incremental compilation disabled. Native timing was not benchmarked.
Linux/Windows cross-checks and live desktop tests were not run for this iOS-only
dependency change. Existing ignored tests remain ignored, not verified.

## Native observations

The QA simulator was iPad Pro 11 (4th), iOS 26.5 (23F77), with English (US) and
Japanese Kana keyboards. Its Connect Hardware Keyboard preference was enabled;
this does not establish a physical keyboard connection. XCUITest key events and
software-key taps drove the actual installed application. A private SSH server
and isolated Herdr terminal recorded received bytes; no personal daemon was used.

Before the patch, Control-Space delivered NUL and Japanese confirmation failed
to deliver the composed text. After the patch, input-source changes were observed
without that extra NUL, and candidate-tap confirmation delivered `日本` once
(`e697a5e69cac`). On the final build, partial and complete software deletion of
`にほん` delivered no text; candidate confirmation delivered `日本` once; ordinary
software Return and Backspace delivered `0d` and `7f`.

Command-A/C/V in an ordinary search field copied and pasted `前日本😀後` exactly
(16 UTF-8 bytes). Arrows with Shift selected the whole emoji. Software deletion
of the trailing `後` and emoji produced `前日本`, verified by copying it back.
Selecting `日本` and pasting `前日本` produced `前前日本`, preserving the unselected
prefix. Terminal paste delivered the same 16-byte fixture. After waiting for the
keyboard resize to settle, terminal Command-A/C copied 122 bytes containing the
recorder heading and Japanese output. Touch taps positioned
the caret; the observed long press did not expose a copy/paste edit menu.

## Simulator limits and remaining acceptance checks

XCUITest Return and Backspace did not reach the app in some hardware-key trials.
A separate standard UIKit `UITextView` comparator also retained `abc` after those
events. Hardware romaji input reached a Japanese candidate popup with Space,
but Return confirmation was not established in either the app or comparator.
Later Control-Space / Shift-Control-Space trials inserted a space as well as
changing the input source: the app received a full-width space and the standard
UITextView retained an ASCII space. Do not infer complete hardware behavior from
the earlier successful switch or from software candidate taps.

Clipboard operations are editing checks, not hardware IME evidence. Interrupted
XCTest runs waiting at the normal iPadOS paste-permission dialog, and runs with
no visible software delete key, were not counted as passing. Permission dialogs
were handled through their normal UI. Host Accessibility/input-monitoring
permissions were not bypassed.

Physical iPad acceptance remains open: the actual keyboard layout, Ctrl-Space,
Shift-Ctrl-Space, Globe, configured Caps Lock switching, hardware Space/Return,
candidate arrows, Escape cancellation, Backspace, selection and clipboard keys,
touch selection/copy, and keyboard hide/show and focus transitions. Repeat both
terminal backends and ordinary fields. Simulator observations do not establish
those physical-key checks. Device installation and startup checks are recorded
separately in SH1-67.

The PRD and evidence record are [SH1-66](https://linear.app/shin1ohno/issue/SH1-66)
and [SH1-67](https://linear.app/shin1ohno/issue/SH1-67). Those builds used the
published earlier fix revision; physical-keyboard acceptance remained open.
