//! A forward's task on the runtime: connect and authenticate as an exec
//! tunnel does, open one `direct-tcpip` channel per request, relay each
//! channel on its own task, and end the local SSH work through the same
//! teardown as a tunnel (`exec::close`).

use std::{sync::Arc, time::Duration};

use russh::{Channel, ChannelOpenFailure, client::Msg};
use tokio::{
    io::{AsyncRead, AsyncWrite, copy_bidirectional_with_sizes},
    net::{TcpStream, UnixStream},
    sync::mpsc::Receiver,
    time::{Instant, sleep_until},
};

use super::{Destination, End, EndSlot, Error, Local, MAX_CHANNELS, Open};
use crate::{
    Failure,
    classify::classify,
    exec::{self, AbortOnDrop, Controls, Login, Session, Started, close},
    teardown::{Fuse, Outcome, SignTracker},
};

/// What the caller waits beyond the task's own deadlines before it gives up
/// on an answer, and how long it waits for the task's recorded end.
pub(super) const ANSWER_MARGIN: Duration = Duration::from_secs(1);
/// Bytes buffered per direction of one relayed connection.
const RELAY_BUFFER: usize = 32 * 1024;
/// The originator a channel names: the caller's end is local to this
/// process, and its real address is not the host's business.
const ORIGINATOR: (&str, u32) = ("127.0.0.1", 0);

pub(super) async fn run(
    login: Login,
    mut requests: Receiver<Open>,
    end_slot: Arc<EndSlot>,
    controls: Controls,
) {
    let fuse = Fuse::new(Arc::clone(&controls.teardown), Outcome::Unproven);
    let signers = SignTracker::default();
    let mut started = Started::default();
    let end = serve(&login, &controls, &signers, &mut started, &mut requests).await;
    match end {
        End::Stopped => tracing::debug!(category = "ssh", "ssh forward stopped"),
        End::Connect(failure) | End::Lost(failure) => {
            tracing::info!(category = "ssh", ?failure, "ssh forward ended");
        }
    }
    // Recorded before the waiting requests go, so a caller that finds its
    // answer gone reads why.
    end_slot.set(end);
    requests.close();
    drop(requests);
    let outcome = close::close(started, login, &signers, &controls).await;
    fuse.settle(outcome);
}

async fn serve(
    login: &Login,
    controls: &Controls,
    signers: &SignTracker,
    started: &mut Started,
    requests: &mut Receiver<Open>,
) -> End {
    if let Err(failure) = exec::connect(login, controls, signers, started).await {
        return match failure {
            Failure::Cancelled => End::Stopped,
            failure => End::Connect(failure),
        };
    }
    loop {
        let Some(session) = started.session.as_mut() else {
            // `connect` keeps the session whenever it succeeds.
            return End::Connect(Failure::Protocol);
        };
        let open = tokio::select! {
            biased;
            () = controls.stop.stopped() => return End::Stopped,
            ended = session.wait_end() => return End::Lost(lost(ended)),
            open = requests.recv() => match open {
                Some(open) => open,
                // Every sender is gone: the forward was dropped.
                None => return End::Stopped,
            },
        };
        let Open {
            destination,
            local,
            reply,
        } = open;
        started.helpers.retain(|relay| !relay.0.is_finished());
        if started.helpers.len() >= MAX_CHANNELS {
            let _ = reply.send(Err(Error::Busy));
            continue;
        }
        let channel = match open_channel(session, destination, controls).await {
            Ok(channel) => channel,
            Err(Opening::Failed(error)) => {
                let _ = reply.send(Err(error));
                continue;
            }
            Err(Opening::Ended(end)) => {
                let _ = reply.send(Err(end.into()));
                return end;
            }
        };
        // A caller that stopped waiting no longer owns the connection.
        if reply.send(Ok(())).is_err() {
            continue;
        }
        tracing::debug!(
            category = "ssh",
            port = destination.port(),
            "ssh forward channel opened"
        );
        started
            .helpers
            .push(AbortOnDrop(tokio::spawn(relay(channel, local))));
    }
}

/// How the session ended, from russh's typed result: a disconnect the host
/// sent counts as the connection going away.
fn lost(ended: Result<(), russh::Error>) -> Failure {
    match ended {
        Ok(()) => Failure::Unreachable,
        Err(error) => classify(&error, None, true),
    }
}

enum Opening {
    /// The channel failed; the connection carries on.
    Failed(Error),
    /// The connection ended while the channel was opening.
    Ended(End),
}

async fn open_channel(
    session: &mut Session,
    destination: Destination,
    controls: &Controls,
) -> Result<Channel<Msg>, Opening> {
    let by = Instant::now() + controls.deadlines.setup;
    let opened = tokio::select! {
        biased;
        () = controls.stop.stopped() => return Err(Opening::Ended(End::Stopped)),
        () = sleep_until(by) => return Err(Opening::Failed(Error::Timeout)),
        opened = session.handle.channel_open_direct_tcpip(
            destination.host().as_str(),
            u32::from(destination.port()),
            ORIGINATOR.0,
            ORIGINATOR.1,
        ) => opened,
    };
    match opened {
        Ok(channel) => Ok(channel),
        // The reason's text, if any, is the host's and is not kept.
        Err(russh::Error::ChannelOpenFailure(reason)) => Err(Opening::Failed(match reason {
            ChannelOpenFailure::AdministrativelyProhibited => Error::Prohibited,
            ChannelOpenFailure::ConnectFailed => Error::PortClosed,
            ChannelOpenFailure::UnknownChannelType
            | ChannelOpenFailure::ResourceShortage
            | ChannelOpenFailure::Other { .. } => Error::Refused,
        })),
        Err(error) => {
            let failure = exec::session_error(session)
                .await
                .unwrap_or_else(|| classify(&error, None, true));
            Err(Opening::Ended(End::Lost(failure)))
        }
    }
}

/// Copies both ways between the caller's connection and the channel until
/// both have ended. EOF from either side reaches the other as EOF; the
/// connection's end or a stop drops both.
async fn relay(channel: Channel<Msg>, local: Local) {
    let remote = channel.into_stream();
    match local {
        Local::Tcp(stream) => {
            if let Ok(stream) = TcpStream::from_std(stream) {
                let _ = stream.set_nodelay(true);
                copy(stream, remote).await;
            }
        }
        Local::Unix(stream) => {
            if let Ok(stream) = UnixStream::from_std(stream) {
                copy(stream, remote).await;
            }
        }
    }
}

async fn copy(
    mut local: impl AsyncRead + AsyncWrite + Unpin,
    mut remote: impl AsyncRead + AsyncWrite + Unpin,
) {
    // An error means one side is gone; dropping both ends the other.
    let _ =
        copy_bidirectional_with_sizes(&mut local, &mut remote, RELAY_BUFFER, RELAY_BUFFER).await;
}
