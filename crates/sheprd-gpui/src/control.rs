//! Lets processes on this machine, such as an agent in a Herdr pane, drive
//! the parts of the app Herdr cannot: show the user a page and hear back
//! what the user noted on it, open the app's own tabs, and read or change
//! state only this client keeps. Herdr itself has no way to reach a client,
//! so the app listens on a socket of its own and `sheprd <command> ...`
//! talks to it. Only this user's local processes can reach it: an agent on
//! an SSH host has no path to this machine's socket.

// The UI shares a few protocol values with the desktop command line. It has
// no command-line listener, socket or helper process on iPadOS.
#[cfg(not(target_os = "ios"))]
mod client;
#[cfg(not(target_os = "ios"))]
mod desktop;
#[cfg(not(target_os = "ios"))]
mod server;

#[cfg(any(not(target_os = "ios"), feature = "cloud"))]
pub(crate) use sheprd_control_protocol::protocol::cloud as batch;
pub use sheprd_control_protocol::protocol::tabs::ZoomChange;

#[cfg(windows)]
pub(crate) use client::send_focus;
#[cfg(all(feature = "mockup", not(target_os = "ios")))]
pub(crate) use client::send_notes;
#[cfg(not(target_os = "ios"))]
pub(crate) use client::{run, send_link, status as exit_status};
#[cfg(any(target_os = "macos", windows))]
pub(crate) use desktop::reload_command;
#[cfg(not(target_os = "ios"))]
pub(crate) use desktop::{Opening, Placed, Target};
#[cfg(not(target_os = "ios"))]
use desktop::{main_windows, plain, plain_path, socket_path, valid_caller};
#[cfg(not(target_os = "ios"))]
pub use protocol::ErrorCode;
#[cfg(not(target_os = "ios"))]
pub(crate) use protocol::{NotesTo, Response};
#[cfg(not(target_os = "ios"))]
pub use protocol::{
    app::{FontSize, Size, Switch},
    events::{Cursor, EventKind},
    settings::Sound,
    work::Script,
};
#[cfg(not(target_os = "ios"))]
pub(crate) use server::install;
#[cfg(not(target_os = "ios"))]
pub(crate) use sheprd_control_protocol::{Error, Placement};
#[cfg(not(target_os = "ios"))]
use sheprd_control_protocol::{protocol, socket};
