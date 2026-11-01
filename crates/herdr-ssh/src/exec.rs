//! One exec tunnel on the runtime: connect, authenticate, open a session
//! channel, run the command, and pump its standard streams.

use std::{collections::VecDeque, sync::Arc, time::Duration};

use russh::{
    Channel, ChannelMsg, ChannelWriteHalf, Disconnect,
    client::{self, AuthResult, Handle, Msg},
};
use tokio::{
    io::AsyncWriteExt as _,
    net::{
        UnixStream,
        unix::{OwnedReadHalf, OwnedWriteHalf},
    },
    task::JoinHandle,
    time::timeout,
};

use crate::{
    ClientKey, Failure, HostKey, Target, TunnelExit,
    classify::classify,
    client_key::SignerError,
    config::{self, CONNECT_DEADLINE, SETUP_DEADLINE},
    handler::{self, Client, OfferedSlot},
};

/// How long to wait for russh's reason once the session has ended.
const SESSION_END_WAIT: Duration = Duration::from_secs(1);
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

/// Runs the tunnel to its end. `record` receives the exit before the
/// caller's stream closes, so a caller that reads end-of-stream finds it.
pub(crate) async fn run(
    request: Request,
    local: UnixStream,
    record: impl FnOnce(TunnelExit) + Send,
) {
    let (local_read, mut local_write) = local.into_split();
    let exit = tunnel(&request, local_read, &mut local_write).await;
    match &exit {
        TunnelExit::Ssh(failure) => tracing::info!(category = "ssh", ?failure, "ssh tunnel failed"),
        TunnelExit::Remote { exit_status, .. } => {
            tracing::debug!(category = "ssh", ?exit_status, "remote command ended");
        }
    }
    record(exit);
    drop(local_write);
}

async fn tunnel(
    request: &Request,
    local_read: OwnedReadHalf,
    local_write: &mut OwnedWriteHalf,
) -> TunnelExit {
    let (mut session, channel) = match open(request).await {
        Ok(opened) => opened,
        Err(failure) => return TunnelExit::Ssh(failure),
    };
    let exit = pump(&mut session, channel, local_read, local_write).await;
    // Dropping the handle ends the session either way; this tells the host.
    let _ = session.disconnect(Disconnect::ByApplication, "", "").await;
    exit
}

async fn open(request: &Request) -> Result<(Handle<Client>, Channel<Msg>), Failure> {
    let host_keys = config::pinned_host_keys(&request.pins);
    if host_keys.is_empty() {
        // Every pin has a type this client cannot verify, so no key the host
        // could offer would match one.
        return Err(Failure::HostKeyChanged);
    }
    let offered = OfferedSlot::default();
    let client = Client::new(request.pins.clone(), true, Arc::clone(&offered));
    let address = (request.target.host.clone(), request.target.port);
    let connecting = client::connect(config::config(host_keys), address, client);
    let mut session = match timeout(CONNECT_DEADLINE, connecting).await {
        Err(_) => return Err(Failure::Timeout),
        Ok(Err(error)) => return Err(classify(&error, handler::take(&offered).as_ref(), true)),
        Ok(Ok(session)) => session,
    };
    match timeout(SETUP_DEADLINE, setup(&mut session, request)).await {
        Err(_) => Err(Failure::Timeout),
        Ok(Err(failure)) => Err(failure),
        Ok(Ok(channel)) => Ok((session, channel)),
    }
}

async fn setup(session: &mut Handle<Client>, request: &Request) -> Result<Channel<Msg>, Failure> {
    let public_key = request.key.public_key().map_err(|_| Failure::Auth)?;
    let mut signer = request.key.signer();
    let user = request.target.user.as_str();
    // Public key only, never a fallback (`BatchMode`). `partial_success`
    // asks for a second method, so it is a failure too.
    match session
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
    let channel = match session.channel_open_session().await {
        Ok(channel) => channel,
        Err(error) => {
            let failure = session_error(session).await;
            return Err(failure.unwrap_or_else(|| classify(&error, None, true)));
        }
    };
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

/// The failure the session ended with, once it has ended with one.
async fn session_error(session: &mut Handle<Client>) -> Option<Failure> {
    if !session.is_closed() {
        return None;
    }
    match timeout(SESSION_END_WAIT, &mut *session).await {
        Ok(Err(error)) => Some(classify(&error, None, true)),
        Ok(Ok(())) | Err(_) => None,
    }
}

/// Aborts a helper task when the tunnel's own task ends or is aborted.
struct AbortOnDrop(JoinHandle<()>);

impl Drop for AbortOnDrop {
    fn drop(&mut self) {
        self.0.abort();
    }
}

async fn pump(
    session: &mut Handle<Client>,
    channel: Channel<Msg>,
    local_read: OwnedReadHalf,
    local_write: &mut OwnedWriteHalf,
) -> TunnelExit {
    let (mut remote, remote_write) = channel.split();
    let upstream = AbortOnDrop(tokio::spawn(forward_input(local_read, remote_write)));
    let mut output = Output::default();
    while let Some(message) = remote.wait().await {
        if let ChannelMsg::Data { data } = message {
            if local_write.write_all(&data).await.is_err() {
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
    drop(upstream);
    match output.finish() {
        Some(exit) => exit,
        None => TunnelExit::Ssh(session_error(session).await.unwrap_or(Failure::Unreachable)),
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
