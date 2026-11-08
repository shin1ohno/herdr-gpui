#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;

mod badge;
#[cfg(all(unix, not(target_os = "ios")))]
mod batch;
mod en_xa;
#[cfg(all(unix, not(target_os = "ios")))]
mod naming;
#[cfg(target_os = "linux")]
mod pinned;
#[cfg(all(unix, not(target_os = "ios")))]
mod process;
#[cfg(all(unix, not(target_os = "ios")))]
mod retry;
#[cfg(all(unix, not(target_os = "ios")))]
mod scan;
#[cfg(all(unix, not(target_os = "ios")))]
mod scan_failures;
mod section;
mod sort;
mod state;

/// A listener of pid `pid` on `number`, started at `started`.
pub(super) fn listener(number: u16, pid: u32, started: u64) -> Listener {
    Listener {
        identity: Identity { pid, started },
        number,
        bind: Bind::Loopback,
        process: "node".into(),
        name: "vite".into(),
        command: "node /home/dev/app/node_modules/.bin/vite".into(),
        directory: Some(PathBuf::from("/home/dev/app")),
        project: Some("app".into()),
        cpu: None,
        memory: 0,
        system: false,
        paused: false,
    }
}
