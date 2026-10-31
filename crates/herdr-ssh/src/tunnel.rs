//! Running a command on a host and exposing its standard streams as a socket
//! pair, and scanning a host's key without authenticating.

use std::{os::unix::net::UnixStream, sync::atomic::AtomicBool, time::Duration};

use crate::{ClientKey, Failure, HostKey, Pins, Result, Target};

/// What a key scan found. The scan disconnects before authentication, so an
/// unverified host never learns the user name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Scan {
    /// The offered key matches a pin.
    Trusted,
    /// Nothing is pinned; these are the keys the host offered, for the user
    /// to compare by fingerprint before `KnownHosts::trust`.
    New(Vec<HostKey>),
    /// A key is pinned and the host offered a different one.
    Changed { offered: HostKey },
}

/// Connects to `target` and reports its key against `pins`, within the
/// connect deadline, observing `stop`. Blocks: call it off the UI thread.
pub fn scan(_target: &Target, _pins: &Pins, _stop: &AtomicBool) -> Result<Scan> {
    todo!("ssh stream")
}

/// How a tunnel ended.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TunnelExit {
    /// The SSH layer failed before or during the command.
    Ssh(Failure),
    /// The remote command ended. `stderr_tail` is at most the last 4 KiB the
    /// command wrote to standard error; it describes the remote program and
    /// never decides an SSH failure class.
    Remote {
        exit_status: Option<u32>,
        stderr_tail: Vec<u8>,
    },
}

/// A running exec channel. Dropping it cancels the channel and the session
/// without blocking.
#[derive(Debug)]
pub struct Tunnel {}

impl Tunnel {
    /// How the tunnel ended, if it has; waits at most `timeout`.
    pub fn wait_exit(&self, _timeout: Duration) -> Option<TunnelExit> {
        todo!("ssh stream")
    }
}

/// Runs `command` on `target` over an exec channel with no PTY and no
/// environment, authenticating with `key` and accepting only a host key in
/// `pins` (which must be [`Pins::Keys`]). Returns at once with this end of
/// a socket pair: the connection proceeds on the runtime, the remote's
/// standard output arrives on the stream, writes go to its standard input,
/// and a failure ends the stream and is reported by [`Tunnel::wait_exit`].
pub fn exec(
    _target: &Target,
    _key: &ClientKey,
    _pins: &Pins,
    _command: &str,
) -> Result<(UnixStream, Tunnel)> {
    todo!("ssh stream")
}
