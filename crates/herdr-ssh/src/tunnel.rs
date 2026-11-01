//! Running a command on a host and exposing its standard streams as a socket
//! pair, and scanning a host's key without authenticating.

use std::{
    os::unix::net::UnixStream,
    sync::{
        Arc, Condvar, Mutex, PoisonError,
        atomic::{AtomicBool, Ordering},
        mpsc::{self, RecvTimeoutError},
    },
    time::Duration,
};

use russh::client;

use crate::{
    ClientKey, Failure, HostKey, Pins, Result, Target,
    classify::classify,
    config::{self, CONNECT_DEADLINE},
    exec::{self, Request},
    handler::{self, Client, Offered, OfferedSlot},
    runtime,
};

/// How often a blocked [`scan`] looks at its stop flag.
const STOP_POLL: Duration = Duration::from_millis(50);

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
pub fn scan(target: &Target, pins: &Pins, stop: &AtomicBool) -> Result<Scan> {
    if stop.load(Ordering::Acquire) {
        return Err(Failure::Cancelled.into());
    }
    let handle = runtime::handle()?;
    let pins = match pins {
        Pins::None => Vec::new(),
        Pins::Keys(keys) => keys.clone(),
    };
    let target = target.clone();
    let (sender, receiver) = mpsc::sync_channel(1);
    let task = handle.spawn(async move {
        // The receiver is gone only when the caller stopped waiting.
        let _ = sender.send(scan_host(&target, pins).await);
    });
    loop {
        if stop.load(Ordering::Acquire) {
            task.abort();
            return Err(Failure::Cancelled.into());
        }
        match receiver.recv_timeout(STOP_POLL) {
            Ok(result) => return result.map_err(Into::into),
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => return Err(Failure::Protocol.into()),
        }
    }
}

async fn scan_host(target: &Target, pins: Vec<HostKey>) -> Result<Scan, Failure> {
    let pinned = !pins.is_empty();
    let offered = OfferedSlot::default();
    let host_keys = config::scan_host_keys(&pins);
    let client = Client::new(pins, false, Arc::clone(&offered));
    let address = (target.host.clone(), target.port);
    let connecting = client::connect(config::config(host_keys), address, client);
    let outcome = tokio::time::timeout(CONNECT_DEADLINE, connecting).await;
    // The check refuses every key, so a recorded key always ends the
    // connection before authentication.
    match handler::take(&offered) {
        Some(Offered::Pinned) => return Ok(Scan::Trusted),
        Some(Offered::New(key)) => return Ok(Scan::New(vec![key])),
        Some(Offered::Changed(key)) => return Ok(Scan::Changed { offered: key }),
        Some(Offered::Refused) => return Err(Failure::HostKeyChanged),
        None => {}
    }
    Err(match outcome {
        Err(_) => Failure::Timeout,
        Ok(Err(error)) => classify(&error, None, pinned),
        // Unreachable while the check refuses every key.
        Ok(Ok(_)) => Failure::Protocol,
    })
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

/// Where the tunnel's task leaves its exit for [`Tunnel::wait_exit`].
#[derive(Debug, Default)]
struct ExitSlot {
    exit: Mutex<Option<TunnelExit>>,
    ended: Condvar,
}

impl ExitSlot {
    fn set(&self, exit: TunnelExit) {
        *self.exit.lock().unwrap_or_else(PoisonError::into_inner) = Some(exit);
        self.ended.notify_all();
    }

    fn wait(&self, timeout: Duration) -> Option<TunnelExit> {
        let exit = self.exit.lock().unwrap_or_else(PoisonError::into_inner);
        let (exit, _) = self
            .ended
            .wait_timeout_while(exit, timeout, |exit| exit.is_none())
            .unwrap_or_else(PoisonError::into_inner);
        exit.clone()
    }
}

/// A running exec channel. Dropping it cancels the channel and the session
/// without blocking.
#[derive(Debug)]
pub struct Tunnel {
    exit: Arc<ExitSlot>,
    task: tokio::task::JoinHandle<()>,
}

impl Tunnel {
    /// How the tunnel ended, if it has; waits at most `timeout`.
    pub fn wait_exit(&self, timeout: Duration) -> Option<TunnelExit> {
        self.exit.wait(timeout)
    }
}

impl Drop for Tunnel {
    /// Aborting only schedules cancellation: the task drops the session and
    /// the stream on the runtime's thread at its next await.
    fn drop(&mut self) {
        self.task.abort();
    }
}

/// Runs `command` on `target` over an exec channel with no PTY and no
/// environment, authenticating with `key` and accepting only a host key in
/// `pins` (which must be [`Pins::Keys`]). Returns at once with this end of
/// a socket pair: the connection proceeds on the runtime, the remote's
/// standard output arrives on the stream, writes go to its standard input,
/// and a failure ends the stream and is reported by [`Tunnel::wait_exit`].
///
/// With [`Pins::None`] nothing is connected and the error is
/// [`Failure::HostKeyUnknown`]: only an explicit scan and trust pin a host.
pub fn exec(
    target: &Target,
    key: &ClientKey,
    pins: &Pins,
    command: &str,
) -> Result<(UnixStream, Tunnel)> {
    let pins = match pins {
        Pins::Keys(keys) if !keys.is_empty() => keys.clone(),
        Pins::Keys(_) | Pins::None => return Err(Failure::HostKeyUnknown.into()),
    };
    let handle = runtime::handle()?;
    let (caller, runtime_end) = UnixStream::pair()?;
    runtime_end.set_nonblocking(true)?;
    let local = {
        let _runtime = handle.enter();
        tokio::net::UnixStream::from_std(runtime_end)?
    };
    let request = Request {
        target: target.clone(),
        key: key.clone(),
        pins,
        command: command.to_owned(),
    };
    let exit = Arc::new(ExitSlot::default());
    let slot = Arc::clone(&exit);
    let task = handle.spawn(exec::run(request, local, move |ended| slot.set(ended)));
    Ok((caller, Tunnel { exit, task }))
}

#[cfg(test)]
mod tests;
