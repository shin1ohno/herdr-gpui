# Herdr UI on iPadOS

Herdr UI runs on iPadOS as the same GPUI app, with a UIKit platform in place
of GPUI's desktop ones and a network transport in place of the local daemon
socket. This document is the design and the contract the parts are built
against.

## Shape

- **Platform.** GPUI has no iOS platform, but `gpui::Application::with_platform`
  takes any `gpui::Platform`. `crates/herdr-uikit` is that platform:
  `UIApplicationMain`, scenes as windows, a `UIView` hosting a `CAMetalLayer`,
  touch, pointer, hardware keyboard, and `UITextInput` input. Its Metal renderer
  and Core Text text system are ported from GPUI's macOS crates
  (`crates/herdr-uikit/NOTICE.md`). Herdr UI calls `herdr_uikit::application()`
  on `target_os = "ios"` where desktop targets call `gpui_platform::application()`.
- **Touch.** GPUI's core already recognizes gestures from raw `TouchEvent`s
  (`gpui-pre/src/window.rs`, `dispatch_touch_event`): taps become clicks, pans
  become scroll with momentum, and long presses and touch drags have their own
  events. The platform delivers raw touches and nothing else.
- **Daemon.** An iPad runs no Herdr daemon and cannot start processes, so it
  reaches a computer's daemon over the network. `herdr_client::connect_with_connector`
  already accepts a connector returning a `UnixStream`; on iOS the connector
  opens a remote tunnel and returns one end of `UnixStream::pair()`, with a
  thread pumping bytes between the pair and the tunnel. Framing, sessions, and
  herdr-core's JSON-RPC are unchanged.
  The tunnel is SSH to the computer, run inside the app ("SSH transport"
  below). An SSH key reaches the user's whole account, so the grant is the
  standard, explicit one: the user adds the iPad's key to `authorized_keys`.
  `herdr-remote-control` is not used: a socket tunnel there would silently
  widen every existing phone pairing from chats to full daemon control.

## Capabilities on iPad

| Area | iPad | Why |
| --- | --- | --- |
| Terminals, panes, tabs, workspaces | Yes | Through the tunnel |
| Sidebar, activity, chat (ACP) | Yes | Daemon state through the tunnel |
| Local daemon, starting `herdr server` or herdr-core | No | No processes on iOS |
| Git status, commit, push, PR creation (`herdr-vcs`) | No | Runs `git` |
| GitHub sign-in and PR prefetch (`herdr-github`) | Later | Device flow is HTTP only; not in the first release |
| Orchestrator dispatch, cloud machines, Teleport | No | Spawn processes or SSH |
| Updater | No | The App Store or Xcode installs the app |
| Browser tabs (`wry`) | No | Links open in Safari |
| Control socket CLI (`herdr-gpui COMMAND`) | No | No agents run on the iPad |
| Saved sign-ins (`herdr-credentials`) | Keychain | A device-only Keychain backend; no plaintext file on iOS |

A feature that is unavailable shows that it is unavailable on iPad. It never
appears to work while doing nothing.

## `herdr-uikit` modules

| Module | Responsibility |
| --- | --- |
| `platform` | `IosPlatform: gpui::Platform`, `UIApplicationMain`, application and scene delegate classes, pasteboard, keyboard layout and mapper, open URL, lifecycle and memory warnings |
| `window` | `IosWindow: gpui::PlatformWindow`, scene bookkeeping, `GPUIView`, touch, pointer hover and scroll, key presses, `UITextInput` |
| `renderer` | Metal renderer, instance buffers, sprite atlas, shaders (`build.rs` generates `scene.h` and stitches the shader source) |
| `text_system` | `IosTextSystem: gpui::PlatformTextSystem` over Core Text and zed-font-kit |
| `dispatcher` | `IosDispatcher: gpui::PlatformDispatcher` over GCD |
| `display`, `display_link` | `IosDisplay: gpui::PlatformDisplay` over `UIScreen`; `CADisplayLink` frame pacing |

The signatures in each module's Wave 0 skeleton are the contract between them:

- `renderer::{Context, InstanceBufferPool, Renderer}`: `Renderer::new(context,
  transparent) -> crate::Result<Renderer>`, `layer_ptr() -> *mut c_void`
  (the `CAMetalLayer`), `sprite_atlas() -> Arc<dyn PlatformAtlas>`,
  `update_drawable_size(Size<DevicePixels>)`, `update_transparency(bool)`,
  `draw(&Scene)`, `destroy()`.
- `text_system::IosTextSystem::new()`, `dispatcher::IosDispatcher::new()`.
- `display::IosDisplay::{primary(), all(), find_by_id(DisplayId)}`.
- `display_link::DisplayLink::{new(mtm, callback) -> crate::Result<_>, start(), stop()}`.
- `window::IosWindow::open(handle, params, foreground_executor,
  renderer_context, mtm) -> anyhow::Result<IosWindow>` and the scene hooks
  `window::{scene_connected, scene_disconnected, scene_activation_changed,
  active_window, window_stack}`.
- `platform::IosPlatform::new()` and `herdr_uikit::application()`.
- `window::input_target::InputTarget`: what keyboard input needs from a
  window, `dispatch(PlatformInput) -> bool`, `with_input_handler(f)`, and
  `scale_factor()`. The window implements it; text input calls it.
- `window::text_input::TextInputView`: an invisible `UIView` adopting
  `UIKeyInput` and `UITextInput` that is first responder while the focused
  element takes text, and receives hardware key presses. The window calls
  `new(mtm, target)`, `view()`, `activate()`, `deactivate()`,
  `input_handler_changed()`, and `caret_bounds_changed(bounds)`. A separate
  view keeps the text system and IME out of the drawing view, as UIKit text
  clients commonly do.

## Scenes and windows

UIKit decides when scenes exist; GPUI decides when windows open. The window
module reconciles them:

1. `UIApplicationMain` starts. The application delegate answers
   `application:configurationForConnectingSceneSession:options:` with a
   configuration whose delegate class is the platform's scene delegate.
2. The first `UIWindowScene` connects. The scene delegate calls
   `window::scene_connected`, then the platform runs GPUI's launch callback,
   whose `open_window` takes that scene.
3. A later `open_window` with no idle scene asks the system for one
   (`UIApplication.activateSceneSessionForRequest`), returns a window that is
   not yet visible, and attaches it when its scene connects.
4. A disconnected scene closes its window; the active scene's window is
   GPUI's active window.

`Info.plist` declares `UIApplicationSceneManifest` with
`UIApplicationSupportsMultipleScenes = true` and no storyboard.

## SSH transport

The desktop reaches a remote Herdr by starting `ssh TARGET "/bin/sh -c
BRIDGE"`, where the bridge script runs the remote `herdr --session S
remote-client-bridge`, and by giving `ssh` one end of `UnixStream::pair()` as
its standard input and output (`herdr-client/src/ssh.rs`, `spawn_bridge`).
The client then reads the bridge's ready line and speaks the ordinary binary
protocol over the other end, with health pings because the connection is
remote. The iPad keeps all of that and replaces only the `ssh` process:

- `crates/herdr-ssh` is an SSH client on russh `=0.65.0`
  (`default-features = false`, `ring`) with one process-wide current-thread
  tokio runtime on its own thread, so no other crate sees async code. It runs
  a command on an exec channel (no PTY, no environment) whose standard streams
  are pumped to one end of a `UnixStream::pair()`. The other end is returned
  at once and the connection proceeds on the runtime, so the bridge's
  existing 15 s ready deadline bounds the whole start, as it does on the
  desktop. The crate builds on every Unix target, so its tests run on macOS
  against a private `sshd`.
- On iOS, `herdr-client`'s `spawn_bridge` uses `herdr-ssh` in place of the
  `ssh` child, and the child guard holds the SSH guard, whose drop only
  signals cancellation (it never blocks or joins). The bridge script, the
  ready handshake, and the health pings are shared with the desktop.
  Failures are not: the desktop reads `ssh`'s exit code and stderr text, so
  `herdr-ssh` reports a typed exit (`TunnelExit::Ssh(SshFailure)` for the
  SSH layer, `TunnelExit::Remote { exit_status, stderr_tail }` for the remote
  command) and the remote's stderr never decides an SSH failure class.
- **The desktop `ssh` policy, kept in process:** public-key authentication
  only, never a fallback, and `partial_success` counts as failure
  (`BatchMode`); 10 s for TCP connect and key exchange (`ConnectTimeout`);
  keepalive every 15 s, 4 misses (`ServerAlive*`); every channel the server
  tries to open (agent, X11, forwarded or direct TCP/streamlocal, session)
  is rejected (`ForwardAgent`, `ForwardX11`, `ClearAllForwardings`); host key
  announcements are ignored (`UpdateHostKeys=no`); no compression.
- **Algorithms** are an explicit allow-list. Key exchange
  `mlkem768x25519-sha256`, `curve25519-sha256`, `curve25519-sha256@libssh.org`
  plus `ext-info-c` and `kex-strict-c-v00@openssh.com` (Terrapin); host keys
  `ssh-ed25519`, `ecdsa-sha2-nistp256/384/521`, `rsa-sha2-512/256` (never
  SHA-1 `ssh-rsa`); ciphers `chacha20-poly1305@openssh.com`,
  `aes256-gcm@openssh.com`; MACs `hmac-sha2-512-etm`, `hmac-sha2-256-etm`.
  Host certificates are refused.
- **Targets** are parsed once at the boundary into the existing
  `Destination { user, host, port }`, which scan, trust, connect, and the
  device list all share. The user is required; IPv6 only as `[addr]:port`,
  without zone ids; host names are ASCII letters, digits, `-`, and `.`,
  lower-cased with any trailing `.` removed (non-ASCII names in punycode).
  The known-hosts name is `host` on port 22 and `[host]:port` otherwise.
  There is no `~/.ssh/config` on iPad.
- **Client key.** One P-256 key per installation, created in the Secure
  Enclave on first use, so the private key never leaves it and is never in
  the app's memory; the app signs through Security.framework, as the
  HerdrRemote app's `DeviceKey.swift` does, and authenticates as
  `ecdsa-sha2-nistp256`, which macOS's `sshd` accepts. Its Keychain item is
  `kSecAttrAccessibleWhenUnlockedThisDeviceOnly`, not synchronizable, and in
  no shared access group. It is created only when a lookup finds none, with
  an add that fails on a duplicate, so two scenes cannot each create one; a
  locked device is a transient error, never a reason to make a new key. The
  simulator and the macOS tests use a software key behind the same signer.
  The Add Device dialog shows the line to add to `~/.ssh/authorized_keys`:
  `restrict ecdsa-sha2-nistp256 AAAA… herdr-ipad-<device>`, since the bridge
  needs no PTY and no forwarding. Settings can replace the key.
- **Host keys** are pinned per known-hosts name in an OpenSSH `known_hosts`
  file under `Library/Application Support`. A host with no pin is unknown;
  a host with any pin that the offered key does not match exactly is a
  changed key and is refused, whatever its type, and connections to a
  pinned host negotiate only the pinned key types. `scan_host_key` stops
  before authentication, so an unverified host never learns the user name.
  `trust_host_key` saves the exact key that was scanned and shown, inside a
  process-wide lock, after re-checking that the host gained no pin meanwhile,
  by writing a temporary file and renaming it. russh's `known_hosts` helpers
  are not used, since they treat a key of another type as unknown.
- **When the user is asked.** Only Add Device and an explicit Connect show a
  host key for approval; a reconnect that meets an unknown or changed key
  stops with the host-key failure (which needs the user) and never prompts.
  A changed key can be resolved from the device's menu, which shows the
  pinned and offered fingerprints side by side and removes the pin only on
  confirmation.
- The skip-verification list has no iPad equivalent.
- The computer needs Remote Login (sshd) enabled and Herdr installed.
- Later: pin the host key out of band, from a QR code the desktop shows,
  instead of trusting the first key the network offers.

## Code rules for the port

- Ported code keeps upstream's structure where it can, so later GPUI snapshots
  can be diffed in, but follows this repository's rules: no `unwrap` or
  `expect` in production, typed errors, files under 1,000 lines.
- `unsafe_code` is allowed per module or per function in FFI code only, and
  every `unsafe` block carries a `// SAFETY:` comment.
- UIKit is touched only with a `MainThreadMarker`.

## Building

`aarch64-apple-ios` is a device, `aarch64-apple-ios-sim` the simulator on an
Apple silicon Mac. `cargo check -p herdr-uikit --target aarch64-apple-ios-sim`
checks the platform; on other targets the crate is empty.

## Running in the simulator

The recipes need macOS with Xcode and its iOS simulator runtime, and the two
Rust targets: `rustup target add aarch64-apple-ios aarch64-apple-ios-sim`.

| Recipe | What it does |
| --- | --- |
| `just lint-ios` | Clippy over `herdr-uikit` and its examples, for a device and for the simulator |
| `just ios-sim EXAMPLE [WAIT]` | Builds the example, bundles it, runs it in the simulator for `WAIT` seconds (default 5), and saves a screenshot |
| `just test-ios` | Runs `herdr-uikit`'s tests inside the simulator, then the screenshot checker's own tests |

Two examples live in `crates/herdr-uikit/examples`:

- `uikit_smoke` is UIKit alone, without GPUI or this crate's modules: a
  magenta window with a white label. When it runs and its screenshot is
  magenta, bundling, installing, launching, logging, and screenshots all work,
  so a failure in `hello` is the platform's.
- `hello` is a GPUI app on `herdr_uikit::application()`: a text line on a
  solid background and a counter that a tap increments.

`just ios-sim uikit_smoke` writes to `target/ios-sim/uikit_smoke/`:
`stdout.log`, `stderr.log` (panics and the examples' messages land here), and
`screenshot.png`. It reports whether the app was still running at the
screenshot and exits with status 4, printing the end of `stderr.log`, when it
was not. The app keeps running afterwards, so it can be used in the
simulator's window (`open -a Simulator`). A cold boot takes about a minute,
and the first launch after it up to two more while the simulator settles;
later runs take seconds.

The scripts behind the recipes are in `scripts/ios/`:

- `bundle.sh` builds an example or binary for `aarch64-apple-ios-sim` (or
  `--target aarch64-apple-ios`), assembles `NAME.app` beside the executable
  from the `Info.plist` template, and ad-hoc signs it. A device bundle is left
  unsigned: installing on an iPad needs a development team and a provisioning
  profile, which the script does not handle yet.
- `sim.sh` boots the simulator when it is shut down, installs and launches the
  app, waits, and takes the screenshot. It never erases, deletes, or shuts
  down a simulator; `--terminate` stops the app it launched after the
  screenshot.
- `sim-runner.sh` is Cargo's runner for `aarch64-apple-ios-sim`: it starts a
  test binary with `xcrun simctl spawn`, which returns the binary's exit
  status. The binary runs outside an app bundle, so tests cannot start
  `UIApplicationMain`.
- `screenshot-color.py` reports the fraction of a screenshot's pixels near a
  color, with the standard library only:
  `python3 -I scripts/ios/screenshot-color.py target/ios-sim/uikit_smoke/screenshot.png ff00ff --min 0.5`.

The simulator is "iPad Pro 13-inch (M5)" by default, the newest runtime's when
several exist. `HERDR_IOS_SIMULATOR` picks another by name or UDID (`xcrun
simctl list devices available`). `RUST_LOG` and `RUST_BACKTRACE` reach the
app; `RUST_*`, `CARGO_*`, and `HERDR_TEST_*` variables reach test binaries.
