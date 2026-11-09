//! Desktop control-socket targeting, window placement and endpoint discovery.
use super::{Placement, protocol, socket};
use crate::{SheprdWindow, browser::Location};
use gpui::App;
use protocol::Caller;
use std::path::{Path, PathBuf};

#[cfg(any(target_os = "macos", windows))]
/// The command that reloads a caller's pages, for the notes prompt.
pub(crate) fn reload_command() -> String {
    static COMMAND: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    COMMAND
        .get_or_init(|| {
            format!(
                "{} browser reload",
                crate::agent_skill::command(crate::agent_skill::executable().as_deref())
            )
        })
        .clone()
}

/// Where this user's app listens: a socket in its state directory, or a
/// named pipe on Windows.
pub(super) fn socket_path() -> crate::Result<PathBuf> {
    let state = crate::preferences::state_dir().ok_or(crate::Error::MissingStateRoot)?;
    Ok(socket::address(&state)?)
}

/// What a request asks the windows to find: a workspace, optionally of one
/// particular local daemon, and the pane asking.
pub(crate) struct Target<'a> {
    pub daemon: Option<&'a Path>,
    pub workspace: Option<&'a str>,
    pub pane: Option<&'a str>,
}

impl Target<'_> {
    /// How strictly a request's workspace is matched, strict first. A caller
    /// that names its daemon is matched on that daemon only: another
    /// session's workspace of the same ID must never be changed for it.
    pub(crate) fn strictness(&self) -> &'static [bool] {
        if self.daemon.is_some() {
            &[true]
        } else {
            &[true, false]
        }
    }
}

/// The page a request asks a window to open, and how to show it.
pub(crate) struct Opening<'a> {
    pub location: &'a Location,
    /// Whether the window switches to the tab.
    pub focus: bool,
    pub placement: Placement,
}

/// Where a window opened a requested tab.
pub(crate) enum Placed {
    Opened {
        workspace_id: String,
        placement: Placement,
    },
    Full,
}

/// Identifiers a request carries are compared, never interpreted, but they
/// are echoed in answers, so they stay short and printable.
pub(super) fn plain(value: &str, limit: usize) -> bool {
    !value.is_empty()
        && value.len() <= limit
        && value.chars().all(|c| !c.is_control() && !c.is_whitespace())
}

/// A daemon socket path: compared and echoed like an identifier, but it may
/// hold spaces, as a Windows profile such as `C:\Users\Jane Doe` does.
pub(super) fn plain_path(value: &str) -> bool {
    !value.is_empty() && value.len() <= 4096 && !value.chars().any(char::is_control)
}

pub(super) fn valid_caller(caller: &Caller) -> bool {
    caller
        .workspace_id
        .as_deref()
        .is_none_or(|id| plain(id, 256))
        && caller.pane_id.as_deref().is_none_or(|id| plain(id, 256))
        && caller.daemon_socket.as_deref().is_none_or(plain_path)
}

pub(super) fn main_windows(cx: &App) -> Vec<gpui::WindowHandle<SheprdWindow>> {
    let active = cx.active_window();
    let mut windows: Vec<_> = cx
        .windows()
        .into_iter()
        .filter_map(|handle| handle.downcast::<SheprdWindow>())
        .collect();
    // The frontmost window wins a workspace that several windows show.
    windows.sort_by_key(|handle| Some(handle.window_id()) != active.map(|a| a.window_id()));
    windows
}

#[cfg(test)]
mod tests;
