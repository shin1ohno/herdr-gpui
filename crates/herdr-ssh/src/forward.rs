//! Carrying TCP connections to a port on the host's loopback over SSH
//! `direct-tcpip` channels (RFC 4254 §7.2), the in-process counterpart of
//! `ssh -L 127.0.0.1:<local>:localhost:<port>`.
//!
//! A [`Forward`] is one dedicated SSH connection: it connects and
//! authenticates exactly as [`exec`](crate::exec) does (the same client key,
//! host-key pins, algorithm allow-list, deadlines, and keepalive) and then
//! opens one `direct-tcpip` channel per connection it is asked to carry.
//! It never asks the host for anything else: no session channel, no remote
//! (`tcpip-forward`) or Unix-socket forwarding, no agent and no X11
//! forwarding, and every channel the host tries to open is refused
//! (`handler`).
//!
//! The destination is always the host's own loopback ([`Destination`]), so
//! a forward cannot be pointed at another machine through the host. Where
//! the local end listens is the caller's business; this module only carries
//! a connection the caller already has ([`Forward::relay`]) or hands back
//! one end of a socket pair ([`Forward::open`]).
//!
//! Failures are typed ([`Error`]): the host refusing to forward
//! ([`Error::Prohibited`]), nothing listening on the port
//! ([`Error::PortClosed`]), and the connection failing to come up
//! ([`Error::Connect`]) or going away later ([`Error::ConnectionLost`]).

use std::{
    net::TcpStream,
    os::unix::net::UnixStream,
    sync::{
        Arc, Condvar, Mutex, PoisonError,
        mpsc::{self, RecvTimeoutError},
    },
    time::Duration,
};

use tokio::sync::mpsc::{Sender, error::TrySendError};

use crate::{
    ClientKey, Failure, Pins, Target,
    exec::{Controls, Deadlines, Login},
    hooks::Hooks,
    runtime,
    teardown::{self, StopSignal, Teardown, TeardownSlot, TunnelId},
};

mod destination;
mod error;
mod session;

pub use destination::{Destination, Loopback};
pub use error::{Error, Result};

/// The most connections one forward carries at once; channels still
/// opening count too. A browser opens about six per host.
pub const MAX_CHANNELS: usize = 32;
/// The most bytes of one carried connection's data that its caller may
/// leave unread before that connection is dropped; see [`Forward::open`].
pub const UNREAD_LIMIT: usize = session::UNREAD_LIMIT;

/// What ended a forward's connection; [`Forward::wait_end`] gives it as an
/// [`Error`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum End {
    Connect(Failure),
    Lost(Failure),
    Stopped,
}

impl From<End> for Error {
    fn from(end: End) -> Self {
        match end {
            End::Connect(failure) => Self::Connect(failure),
            End::Lost(failure) => Self::ConnectionLost(failure),
            End::Stopped => Self::Stopped,
        }
    }
}

/// Where the forward's task leaves its end for the forward's owner.
#[derive(Debug, Default)]
struct EndSlot {
    end: Mutex<Option<End>>,
    ended: Condvar,
}

impl EndSlot {
    fn set(&self, end: End) {
        *self.end.lock().unwrap_or_else(PoisonError::into_inner) = Some(end);
        self.ended.notify_all();
    }

    fn wait(&self, timeout: Duration) -> Option<End> {
        let end = self.end.lock().unwrap_or_else(PoisonError::into_inner);
        let (end, _) = self
            .ended
            .wait_timeout_while(end, timeout, |end| end.is_none())
            .unwrap_or_else(PoisonError::into_inner);
        *end
    }
}

/// The caller's end of one forwarded connection, handed to the task.
enum Local {
    Tcp(TcpStream),
    Unix(UnixStream),
}

/// One request to carry a connection, answered once its channel is open or
/// has failed.
struct Open {
    destination: Destination,
    local: Local,
    reply: mpsc::SyncSender<Result<()>>,
}

/// One SSH connection that carries TCP connections to the host's loopback.
/// Dropping it stops it without blocking: its task closes every carried
/// connection, disconnects, and ends its local work on its own. To know
/// that work has ended, call [`Forward::stop`] and then
/// [`Forward::wait_teardown`] before dropping it.
#[derive(Debug)]
pub struct Forward {
    end: Arc<EndSlot>,
    id: TunnelId,
    /// How long a caller waits for its channel: the connection's own
    /// deadlines plus the channel's, so the task always answers first.
    patience: Duration,
    requests: Sender<Open>,
    stop: Arc<StopSignal>,
    teardown: Arc<TeardownSlot>,
}

impl Forward {
    /// Starts a forwarding connection to `target`, authenticating with `key`
    /// and accepting only a host key in `pins` (which must be
    /// [`Pins::Keys`]). Returns at once: the connection comes up on the
    /// runtime, and the first [`Forward::open`] or [`Forward::relay`] waits
    /// for it and reports its failure.
    ///
    /// With [`Pins::None`] nothing is connected and the error is
    /// [`Error::Connect`] with [`Failure::HostKeyUnknown`].
    pub fn connect(target: &Target, key: &ClientKey, pins: &Pins) -> Result<Self> {
        Self::start(target, key, pins, Deadlines::default(), Hooks::default())
    }

    /// [`Forward::connect`] with its deadlines and test hooks. Every `Err`
    /// comes before the task is spawned, so nothing reached the host.
    pub(crate) fn start(
        target: &Target,
        key: &ClientKey,
        pins: &Pins,
        deadlines: Deadlines,
        hooks: Hooks,
    ) -> Result<Self> {
        let pins = match pins {
            Pins::Keys(keys) if !keys.is_empty() => keys.clone(),
            Pins::Keys(_) | Pins::None => return Err(Error::Connect(Failure::HostKeyUnknown)),
        };
        let id = teardown::next_tunnel_id().map_err(std::io::Error::other)?;
        let handle = runtime::handle()?;
        let (requests, received) = tokio::sync::mpsc::channel(MAX_CHANNELS);
        let end = Arc::new(EndSlot::default());
        let stop = Arc::new(StopSignal::default());
        let teardown = Arc::new(TeardownSlot::default());
        let login = Login {
            target: target.clone(),
            key: key.clone(),
            pins,
        };
        let controls = Controls {
            stop: Arc::clone(&stop),
            teardown: Arc::clone(&teardown),
            deadlines,
            hooks,
        };
        // The task is never aborted: a stop or the last sender's drop ends
        // it through its teardown.
        drop(handle.spawn(session::run(login, received, Arc::clone(&end), controls)));
        Ok(Self {
            end,
            id,
            patience: deadlines.connect
                + deadlines.setup
                + deadlines.setup
                + session::ANSWER_MARGIN,
            requests,
            stop,
            teardown,
        })
    }

    /// Opens a channel to `destination` and returns this end of a socket
    /// pair that carries it: what is written to it reaches the host's port,
    /// what the host's port sends arrives on it, and shutting down its write
    /// side sends EOF. Blocks until the channel is open or has failed: call
    /// it off the UI thread. Channels open concurrently, so a slow port
    /// delays only its own request.
    ///
    /// The host's data is always taken off the connection as it arrives, so
    /// a caller that stops reading cannot stall the forward's other
    /// connections; once it leaves more than [`UNREAD_LIMIT`] bytes unread,
    /// its connection alone is dropped.
    pub fn open(&self, destination: Destination) -> Result<UnixStream> {
        let (caller, task_end) = UnixStream::pair()?;
        task_end.set_nonblocking(true)?;
        self.request(destination, Local::Unix(task_end))?;
        Ok(caller)
    }

    /// Opens a channel to `destination` and carries `stream` over it, for
    /// example a connection accepted on a local loopback listener; the
    /// stream is closed when either side ends. Blocks until the channel is
    /// open or has failed: call it off the UI thread. On `Err` the stream
    /// has been dropped.
    pub fn relay(&self, destination: Destination, stream: TcpStream) -> Result<()> {
        stream.set_nonblocking(true)?;
        self.request(destination, Local::Tcp(stream))
    }

    fn request(&self, destination: Destination, local: Local) -> Result<()> {
        let (reply, answer) = mpsc::sync_channel(1);
        let open = Open {
            destination,
            local,
            reply,
        };
        match self.requests.try_send(open) {
            Ok(()) => {}
            Err(TrySendError::Full(_)) => return Err(Error::Busy),
            Err(TrySendError::Closed(_)) => return Err(self.ended()),
        }
        match answer.recv_timeout(self.patience) {
            Ok(result) => result,
            Err(RecvTimeoutError::Timeout) => Err(Error::Timeout),
            // The task records its end before it drops waiting requests.
            Err(RecvTimeoutError::Disconnected) => Err(self.ended()),
        }
    }

    fn ended(&self) -> Error {
        self.end
            .wait(session::END_WAIT)
            .map_or(Error::Stopped, Error::from)
    }

    /// Why the forwarding connection ended, once it has: waits at most
    /// `timeout`. [`Error::Connect`] means it never came up,
    /// [`Error::ConnectionLost`] that it went away after, and
    /// [`Error::Stopped`] that its owner stopped or dropped it.
    pub fn wait_end(&self, timeout: Duration) -> Option<Error> {
        self.end.wait(timeout).map(Error::from)
    }

    /// This forward's identity, unique within the process among forwards
    /// and tunnels; its receipt carries the same one.
    pub fn id(&self) -> TunnelId {
        self.id
    }

    /// Asks the forward to end: no new channel opens, every carried
    /// connection is closed, and the connection disconnects. Never blocks.
    pub fn stop(&self) {
        self.stop.stop();
    }

    /// How the forward's local SSH work ended, once known; waits at most
    /// `timeout`. See [`TeardownReceipt`](crate::TeardownReceipt) for what
    /// `Complete` shows.
    pub fn wait_teardown(&self, timeout: Duration) -> Option<Teardown> {
        self.teardown.wait(self.id, timeout)
    }
}

impl Drop for Forward {
    fn drop(&mut self) {
        self.stop();
    }
}

#[cfg(test)]
mod tests;
