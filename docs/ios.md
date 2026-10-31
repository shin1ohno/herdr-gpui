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
