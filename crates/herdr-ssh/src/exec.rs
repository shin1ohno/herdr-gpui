//! One exec tunnel on the runtime: connect, authenticate, open a session
//! channel, run the command, pump its standard streams, and end the local
//! SSH work so that a [`TeardownReceipt`](crate::TeardownReceipt) can vouch
//! for it (`close`).

use std::{collections::VecDeque, sync::Arc, time::Duration};

use russh::{
    Channel, ChannelMsg, ChannelWriteHalf,
    client::{self, AuthResult, Handle, Msg},
};
use tokio::{
    io::AsyncWriteExt as _,
    net::{
        TcpStream, UnixStream,
        unix::{OwnedReadHalf, OwnedWriteHalf},
    },
    task::JoinHandle,
    time::{Instant, sleep_until, timeout},
};

use crate::{
    ClientKey, Failure, HostKey, Target, TunnelExit,
    classify::classify,
    client_key::SignerError,
    config::{self, CONNECT_DEADLINE, SETUP_DEADLINE},
    handler::{self, Client, OfferedSlot},
    hooks::{Event, Holder, Hooks, Mutant},
    teardown::{Fuse, Outcome, SignTracker, StopSignal, TeardownSlot},
    transport::{KillSwitch, Killable},
};

mod close;

/// How long to wait for russh's reason once the session has ended.
const SESSION_END_WAIT: Duration = Duration::from_secs(1);
/// How long the teardown waits for the session task to end, and separately
/// for running signing closures, before it settles `Unproven`.
pub(crate) const TEARDOWN_DEADLINE: Duration = Duration::from_secs(5);
/// How long the host gets to act on the disconnect before the teardown cuts
/// the transport, within [`TEARDOWN_DEADLINE`].
const DISCONNECT_GRACE: Duration = Duration::from_secs(1);
/// The most standard error kept, as the desktop keeps of `ssh`'s.
pub(crate) const STDERR_TAIL: usize = 4096;
/// The extended data stream that carries standard error (RFC 4254).
const STDERR: u32 = 1;

pub(crate) struct Request {
    pub(crate) target: Target,
    pub(crate) key: ClientKey,
    pub(crate) pins: Vec<HostKey>,
    pub(crate) command: String,
}

/// The tunnel's deadlines; tests shorten them.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Deadlines {
    pub(crate) connect: Duration,
    pub(crate) setup: Duration,
    pub(crate) disconnect: Duration,
    /// For the session task, from the start of the teardown.
    pub(crate) join: Duration,
    /// For running signing closures, after the session task has ended.
    pub(crate) signers: Duration,
}

impl Default for Deadlines {
    fn default() -> Self {
        Self {
            connect: CONNECT_DEADLINE,
            setup: SETUP_DEADLINE,
            disconnect: DISCONNECT_GRACE,
            join: TEARDOWN_DEADLINE,
            signers: TEARDOWN_DEADLINE,
        }
    }
}

/// What the tunnel's task shares with its [`Tunnel`](crate::Tunnel).
pub(crate) struct Controls {
    pub(crate) stop: Arc<StopSignal>,
    pub(crate) teardown: Arc<TeardownSlot>,
    pub(crate) deadlines: Deadlines,
    pub(crate) hooks: Hooks,
}

/// What the tunnel started and must end before it can give a receipt.
#[derive(Default)]
struct Started {
    /// Cuts the stream the session task owns.
    transport: Option<Arc<KillSwitch>>,
    session: Option<Session>,
    /// A session task may run that this tunnel can no longer join.
    unjoined: bool,
    input: Option<AbortOnDrop>,
}

/// Runs the tunnel to its end. `record` receives the exit before the
/// caller's stream closes, so a caller that reads end-of-stream finds it.
/// The teardown outcome is settled last, after the stream has closed.
pub(crate) async fn run(
    request: Request,
    local: UnixStream,
    record: impl FnOnce(TunnelExit) + Send,
    controls: Controls,
) {
    let on_drop = if controls.hooks.removes(Mutant::CompleteOnPanic) {
        Outcome::Complete
    } else {
        Outcome::Unproven
    };
    let fuse = Fuse::new(Arc::clone(&controls.teardown), on_drop);
    let signers = SignTracker::default();
    let mut started = Started::default();
    let (local_read, mut local_write) = local.into_split();
    let exit = tunnel(
        &request,
        &controls,
        &signers,
        &mut started,
        local_read,
        &mut local_write,
    )
    .await;
    match &exit {
        TunnelExit::Ssh(failure) => tracing::info!(category = "ssh", ?failure, "ssh tunnel failed"),
        TunnelExit::Remote { exit_status, .. } => {
            tracing::debug!(category = "ssh", ?exit_status, "remote command ended");
        }
    }
    if controls.hooks.removes(Mutant::ReceiptBeforeExit) {
        let outcome = close::close(started, request, &signers, &controls).await;
        fuse.settle(outcome);
        record(exit);
        controls.hooks.record(Event::ExitRecorded);
        return;
    }
    record(exit);
    controls.hooks.record(Event::ExitRecorded);
    drop(local_write);
    let outcome = close::close(started, request, &signers, &controls).await;
    fuse.settle(outcome);
}

async fn tunnel(
    request: &Request,
    controls: &Controls,
    signers: &SignTracker,
    started: &mut Started,
    local_read: OwnedReadHalf,
    local_write: &mut OwnedWriteHalf,
) -> TunnelExit {
    let channel = match open(request, controls, signers, started).await {
        Ok(channel) => channel,
        Err(failure) => return TunnelExit::Ssh(failure),
    };
    pump(controls, started, channel, local_read, local_write).await
}

async fn open(
    request: &Request,
    controls: &Controls,
    signers: &SignTracker,
    started: &mut Started,
) -> Result<Channel<Msg>, Failure> {
    let host_keys = config::pinned_host_keys(&request.pins);
    if host_keys.is_empty() {
        // Every pin has a type this client cannot verify, so no key the host
        // could offer would match one.
        return Err(Failure::HostKeyChanged);
    }
    let stop = &controls.stop;
    let connect_by = Instant::now() + controls.deadlines.connect;
    // Until the TCP connection exists nothing of SSH has started, so this
    // future may be dropped.
    let address = (request.target.host.as_str(), request.target.port);
    let socket = tokio::select! {
        biased;
        () = stop.stopped() => return Err(Failure::Cancelled),
        () = sleep_until(connect_by) => return Err(Failure::Timeout),
        socket = TcpStream::connect(address) => {
            socket.map_err(|error| classify(&russh::Error::IO(error), None, true))?
        }
    };
    // As russh's own `connect` does with `Config::nodelay`; a failure only
    // costs latency.
    let _ = socket.set_nodelay(true);
    let transport = Arc::new(KillSwitch::new(controls.hooks.deaf_transport()));
    started.transport = Some(Arc::clone(&transport));
    let offered = OfferedSlot::default();
    let client = Client::new(request.pins.clone(), true, Arc::clone(&offered))
        .holding(controls.hooks.holder(Holder::Handler));
    let stream = Killable::new(socket, Arc::clone(&transport));
    let connecting = client::connect_stream(config::config(host_keys), stream, client);
    tokio::pin!(connecting);
    let connected = tokio::select! {
        biased;
        () = stop.stopped() => Err(Failure::Cancelled),
        () = sleep_until(connect_by) => Err(Failure::Timeout),
        connected = &mut connecting => Ok(connected),
    };
    let connected = match connected {
        Ok(connected) => connected,
        Err(cut) => {
            if controls.hooks.removes(Mutant::DropConnectingOnCut) {
                // The mutant drops the handshake, leaving any session task
                // russh spawned to run on with the stream.
                return Err(cut);
            }
            // russh may already run the session task, which only this future
            // can join, so it is driven to its end rather than dropped:
            // cutting the stream fails the handshake at once.
            transport.kill();
            match timeout(controls.deadlines.join, &mut connecting).await {
                // It finished meanwhile; the teardown disconnects and joins.
                Ok(Ok(handle)) => started.session = Some(Session::new(handle)),
                Ok(Err(_)) => {}
                Err(_) => started.unjoined = true,
            }
            return Err(cut);
        }
    };
    let session = match connected {
        Ok(handle) => started.session.insert(Session::new(handle)),
        Err(error) => return Err(classify(&error, handler::take(&offered).as_ref(), true)),
    };
    // From here the session task is joined through the handle, so the
    // setup future may be dropped on a stop or its deadline.
    let setup_by = Instant::now() + controls.deadlines.setup;
    let observe_stop = !controls.hooks.removes(Mutant::IgnoreStopInSetup);
    tokio::select! {
        biased;
        () = stop.stopped(), if observe_stop => Err(Failure::Cancelled),
        () = sleep_until(setup_by) => Err(Failure::Timeout),
        channel = setup(session, request, controls, signers) => channel,
    }
}

async fn setup(
    session: &mut Session,
    request: &Request,
    controls: &Controls,
    signers: &SignTracker,
) -> Result<Channel<Msg>, Failure> {
    let public_key = request.key.public_key().map_err(|_| Failure::Auth)?;
    let mut signer = request.key.signer(signers.clone());
    let user = request.target.user.as_str();
    // Public key only, never a fallback (`BatchMode`). `partial_success`
    // asks for a second method, so it is a failure too.
    match session
        .handle
        .authenticate_publickey_with(user, public_key, None, &mut signer)
        .await
    {
        Ok(AuthResult::Success) => {}
        Ok(AuthResult::Failure { .. }) => {
            return Err(session_error(session).await.unwrap_or(Failure::Auth));
        }
        Err(SignerError::Send(_)) => {
            return Err(session_error(session).await.unwrap_or(Failure::Unreachable));
        }
        Err(SignerError::Sign(_) | SignerError::Join(_)) => return Err(Failure::Auth),
    }
    let channel = match session.handle.channel_open_session().await {
        Ok(channel) => channel,
        Err(error) => {
            let failure = session_error(session).await;
            return Err(failure.unwrap_or_else(|| classify(&error, None, true)));
        }
    };
    // Checked before the request is queued, so a stop that arrived while the
    // channel opened sends nothing.
    if controls.stop.is_stopped() && !controls.hooks.removes(Mutant::IgnoreStopInSetup) {
        return Err(Failure::Cancelled);
    }
    controls.hooks.record(Event::ExecSent);
    // No PTY and no environment (`ssh -T`). The reply arrives on the
    // channel, where the pump reads it.
    if channel
        .exec(true, request.command.as_bytes())
        .await
        .is_err()
    {
        return Err(session_error(session).await.unwrap_or(Failure::Unreachable));
    }
    Ok(channel)
}

/// russh's handle, and whether the session task's end has been observed
/// through it. The handle's join is a oneshot receiver that panics when
/// polled again after it completed, and both the failure classification and
/// the teardown wait for the end, so every wait goes through
/// [`Session::wait_end`], which polls it at most until it completes.
struct Session {
    handle: Handle<Client>,
    ended: bool,
}

impl Session {
    fn new(handle: Handle<Client>) -> Self {
        Self {
            handle,
            ended: false,
        }
    }

    /// Resolves when the session task has ended: with its result the first
    /// time, and at once with `Ok` after that (the result was taken).
    /// Cancelling it before the end leaves the join unpolled to completion.
    async fn wait_end(&mut self) -> Result<(), russh::Error> {
        if self.ended {
            return Ok(());
        }
        let result = (&mut self.handle).await;
        // Set in the same poll that completed the join, so no cancellation
        // can fall between the two.
        self.ended = true;
        result
    }
}

/// The failure the session ended with, once it has ended with one.
async fn session_error(session: &mut Session) -> Option<Failure> {
    if !session.handle.is_closed() {
        return None;
    }
    match timeout(SESSION_END_WAIT, session.wait_end()).await {
        Ok(Err(error)) => Some(classify(&error, None, true)),
        Ok(Ok(())) | Err(_) => None,
    }
}

/// Aborts a helper task when the tunnel's own task ends or is aborted.
struct AbortOnDrop(JoinHandle<()>);

impl AbortOnDrop {
    /// Aborts the task and waits until it has ended and dropped what it held.
    async fn end(mut self) {
        self.0.abort();
        let _ = (&mut self.0).await;
    }
}

impl Drop for AbortOnDrop {
    fn drop(&mut self) {
        self.0.abort();
    }
}

async fn pump(
    controls: &Controls,
    started: &mut Started,
    channel: Channel<Msg>,
    local_read: OwnedReadHalf,
    local_write: &mut OwnedWriteHalf,
) -> TunnelExit {
    let (mut remote, remote_write) = channel.split();
    let held = controls.hooks.holder(Holder::Forwarder);
    started.input = Some(AbortOnDrop(tokio::spawn(async move {
        let _held = held;
        forward_input(local_read, remote_write).await;
    })));
    let stop = &controls.stop;
    let mut output = Output::default();
    loop {
        let message = tokio::select! {
            biased;
            () = stop.stopped() => return TunnelExit::Ssh(Failure::Cancelled),
            message = remote.wait() => message,
        };
        let Some(message) = message else { break };
        if controls.hooks.pump_panics() {
            panic!("test fault point: the pump panics");
        }
        if let ChannelMsg::Data { data } = message {
            // A caller that stopped reading must not hold the stop.
            let written = tokio::select! {
                biased;
                () = stop.stopped() => return TunnelExit::Ssh(Failure::Cancelled),
                written = local_write.write_all(&data) => written,
            };
            if written.is_err() {
                // The caller closed its end, so nobody reads the rest.
                return TunnelExit::Ssh(Failure::Cancelled);
            }
            continue;
        }
        match output.receive(message) {
            // EOF may precede exit-status and stderr. Keep the caller's
            // stream open until run records the typed exit, otherwise its
            // worker can observe EOF before the failure classification.
            Step::Continue | Step::Eof => {}
            Step::Refused => return TunnelExit::Ssh(Failure::Protocol),
            Step::Closed => break,
        }
    }
    match output.finish() {
        Some(exit) => exit,
        None => {
            let failure = match started.session.as_mut() {
                Some(session) => session_error(session).await,
                None => None,
            };
            TunnelExit::Ssh(failure.unwrap_or(Failure::Unreachable))
        }
    }
}

/// Copies the caller's writes to the command's standard input. The caller's
/// end of input reaches the command as EOF.
async fn forward_input(mut local: OwnedReadHalf, remote: ChannelWriteHalf<Msg>) {
    let mut writer = remote.make_writer();
    if tokio::io::copy(&mut local, &mut writer).await.is_ok() {
        // An error means the channel is gone, which the pump reports.
        let _ = writer.shutdown().await;
    }
}

/// What the channel said besides standard output.
#[derive(Debug, Default)]
pub(crate) struct Output {
    /// The host accepted the exec request.
    accepted: bool,
    closed: bool,
    exit_status: Option<u32>,
    /// The command was killed by a signal; its name and message are not kept.
    signalled: bool,
    stderr: Tail,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Step {
    Continue,
    /// The command closed its standard output.
    Eof,
    /// The host refused to run the command.
    Refused,
    Closed,
}

impl Output {
    pub(crate) fn receive(&mut self, message: ChannelMsg) -> Step {
        match message {
            ChannelMsg::ExtendedData { data, ext: STDERR } => self.stderr.push(&data),
            ChannelMsg::Eof => return Step::Eof,
            ChannelMsg::ExitStatus { exit_status } => self.exit_status = Some(exit_status),
            ChannelMsg::ExitSignal { .. } => self.signalled = true,
            ChannelMsg::Success => self.accepted = true,
            ChannelMsg::Failure if !self.accepted => return Step::Refused,
            ChannelMsg::Close => {
                self.closed = true;
                return Step::Closed;
            }
            _ => {}
        }
        Step::Continue
    }

    /// How the command ended, once the channel says it did; `None` when the
    /// session went away first, which is an SSH failure.
    pub(crate) fn finish(self) -> Option<TunnelExit> {
        (self.closed || self.exit_status.is_some() || self.signalled).then(|| TunnelExit::Remote {
            exit_status: self.exit_status,
            stderr_tail: self.stderr.into_vec(),
        })
    }
}

/// The last [`STDERR_TAIL`] bytes written to it.
#[derive(Debug, Default)]
pub(crate) struct Tail(VecDeque<u8>);

impl Tail {
    pub(crate) fn push(&mut self, data: &[u8]) {
        let data = data
            .get(data.len().saturating_sub(STDERR_TAIL)..)
            .unwrap_or(data);
        let overflow = (self.0.len() + data.len()).saturating_sub(STDERR_TAIL);
        self.0.drain(..overflow);
        self.0.extend(data);
    }

    pub(crate) fn into_vec(self) -> Vec<u8> {
        self.0.into()
    }
}

#[cfg(test)]
mod tests;
