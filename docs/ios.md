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
  - Stage 1: a tunnel through `herdr-remote-control` on the computer, over the
    HTTPS endpoint and pairing it already has.
  - Stage 2: in-process SSH to the computer's daemon socket.

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
| Saved sign-ins (`herdr-credentials`) | Keychain | Security.framework exists on iOS |

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
