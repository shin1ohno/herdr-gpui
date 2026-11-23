//! A forward's task on the runtime: connect and authenticate as an exec
//! tunnel does, open `direct-tcpip` channels concurrently, relay each channel
//! on its own task, and end the local SSH work through the same teardown as
//! a tunnel (`exec::close`).
//!
//! Two rules keep one connection from stalling every carried connection:
//!
//! - Opens run concurrently, so a host port that is slow to answer delays
//!   only its own request.
//! - A relay always takes what its channel delivers. russh 0.65 grants the
//!   host more window as soon as data arrives (`client/encrypted.rs`), so a
//!   channel's only backpressure is its bounded queue into the session task,
//!   and a full queue stops the session task for every channel and for
//!   keepalives. A caller that stops reading therefore never fills that
//!   queue: its relay keeps up to [`UNREAD_LIMIT`] bytes and then resets
//!   that one connection (RST, so the caller sees an abnormal end).

use std::{
    collections::VecDeque,
    future::{Future, poll_fn},
    pin::Pin,
    sync::Arc,
    task::Poll,
    time::Duration,
};

use russh::{
    Channel, ChannelMsg, ChannelOpenFailure, ChannelReadHalf,
    client::{Handle, Msg},
};
use tokio::{
    io::{AsyncWrite, AsyncWriteExt as _, BufReader, copy_buf},
    net::TcpStream,
    sync::mpsc::Receiver,
    time::timeout,
};

use super::{Destination, End, EndSlot, Error, MAX_CHANNELS, Open};
use crate::{
    Failure,
    classify::classify,
    exec::{self, AbortOnDrop, Controls, Login, Started, close},
    handler::Client,
    teardown::{Fuse, Outcome, SignTracker, StopSignal},
    transport::KillSwitch,
};

/// How long a caller whose answer went away waits for the task to record
/// why: the task may first wait up to a second for russh's reason.
pub(super) const END_WAIT: Duration = Duration::from_secs(3);
/// What the caller waits beyond the task's own deadlines before it gives up
/// on an answer.
pub(super) const ANSWER_MARGIN: Duration = Duration::from_secs(1);
/// The most bytes of one channel's data that its caller has not read yet.
/// Past it, that connection is dropped rather than letting it hold the
/// session task (see the module comment).
pub(crate) const UNREAD_LIMIT: usize = 2 * 1024 * 1024;
/// Bytes read from the caller per write to the channel.
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
    let Started {
        session,
        helpers,
        transport,
        ..
    } = started;
    // `connect` keeps both whenever it succeeds.
    let (Some(session), Some(transport)) = (session.as_mut(), transport.clone()) else {
        return End::Connect(Failure::Protocol);
    };
    let carried = carry(
        &session.handle,
        &transport,
        helpers,
        requests,
        &controls.stop,
        controls.deadlines.setup,
    )
    .await;
    match carried {
        Carried::Stopped => End::Stopped,
        Carried::Lost(hint) => End::Lost(
            exec::session_error(session)
                .await
                .or(hint)
                .unwrap_or(Failure::Unreachable),
        ),
    }
}

enum Carried {
    Stopped,
    /// The session ended; the failure an open saw, if one did.
    Lost(Option<Failure>),
}

/// A channel being opened. It borrows the session's handle, so opens run
/// concurrently on this task rather than on tasks of their own.
type Opening<'a> = Pin<Box<dyn Future<Output = Opened> + Send + 'a>>;

/// A request whose channel has opened or failed.
struct Opened {
    destination: Destination,
    local: std::net::TcpStream,
    reply: std::sync::mpsc::SyncSender<super::Result<()>>,
    result: Result<Channel<Msg>, Failed>,
}

enum Failed {
    /// The host or the deadline refused this channel; the connection
    /// carries on.
    Channel(Error),
    /// The session went away while the channel was opening.
    Session(russh::Error),
}

/// Takes requests and carries connections until a stop, the forward's
/// drop, or the session's end. At most [`MAX_CHANNELS`] channels are open
/// or opening at once.
async fn carry(
    handle: &Handle<Client>,
    transport: &KillSwitch,
    relays: &mut Vec<AbortOnDrop>,
    requests: &mut Receiver<Open>,
    stop: &StopSignal,
    deadline: Duration,
) -> Carried {
    let mut opening: Vec<Opening<'_>> = Vec::new();
    loop {
        tokio::select! {
            biased;
            () = stop.stopped() => return Carried::Stopped,
            // The session task owns the stream, so its drop is the session's
            // end, seen without borrowing the handle mutably.
            () = transport.wait_released() => return Carried::Lost(None),
            opened = next(&mut opening), if !opening.is_empty() => {
                if let Some(failure) = finish(opened, relays).await {
                    return Carried::Lost(Some(failure));
                }
            }
            open = requests.recv() => {
                // Every sender is gone: the forward was dropped.
                let Some(open) = open else { return Carried::Stopped };
                relays.retain(|relay| !relay.0.is_finished());
                if relays.len() + opening.len() >= MAX_CHANNELS {
                    let _ = open.reply.send(Err(Error::Busy));
                } else {
                    opening.push(Box::pin(open_channel(handle, open, deadline)));
                }
            }
        }
    }
}

/// The first of `opening` to finish, removed from it.
async fn next(opening: &mut Vec<Opening<'_>>) -> Opened {
    poll_fn(|cx| {
        for index in 0..opening.len() {
            if let Poll::Ready(opened) = opening[index].as_mut().poll(cx) {
                drop(opening.swap_remove(index));
                return Poll::Ready(opened);
            }
        }
        Poll::Pending
    })
    .await
}

async fn open_channel(handle: &Handle<Client>, open: Open, deadline: Duration) -> Opened {
    let Open {
        destination,
        local,
        reply,
    } = open;
    let opened = timeout(
        deadline,
        handle.channel_open_direct_tcpip(
            destination.host().as_str(),
            u32::from(destination.port()),
            ORIGINATOR.0,
            ORIGINATOR.1,
        ),
    )
    .await;
    let result = match opened {
        Err(_) => Err(Failed::Channel(Error::Timeout)),
        Ok(Ok(channel)) => Ok(channel),
        // The reason's text, if any, is the host's and is not kept.
        Ok(Err(russh::Error::ChannelOpenFailure(reason))) => Err(Failed::Channel(match reason {
            ChannelOpenFailure::AdministrativelyProhibited => Error::Prohibited,
            ChannelOpenFailure::ConnectFailed => Error::PortClosed,
            ChannelOpenFailure::UnknownChannelType
            | ChannelOpenFailure::ResourceShortage
            | ChannelOpenFailure::Other { .. } => Error::Refused,
        })),
        Ok(Err(error)) => Err(Failed::Session(error)),
    };
    Opened {
        destination,
        local,
        reply,
        result,
    }
}

/// Answers the request and starts its relay; the failure when the session
/// went away.
async fn finish(opened: Opened, relays: &mut Vec<AbortOnDrop>) -> Option<Failure> {
    let Opened {
        destination,
        local,
        reply,
        result,
    } = opened;
    match result {
        Ok(channel) => {
            if reply.send(Ok(())).is_err() {
                // The caller stopped waiting and no longer owns the
                // connection: the host's end is closed now, not when the
                // session ends.
                let _ = channel.close().await;
                return None;
            }
            tracing::debug!(
                category = "ssh",
                port = destination.port(),
                "ssh forward channel opened"
            );
            relays.push(AbortOnDrop(tokio::spawn(relay(
                channel,
                local,
                destination.port(),
            ))));
            None
        }
        Err(Failed::Channel(error)) => {
            let _ = reply.send(Err(error));
            None
        }
        // The reply goes with the request; its caller reads the end.
        Err(Failed::Session(error)) => Some(classify(&error, None, true)),
    }
}

async fn relay(channel: Channel<Msg>, local: std::net::TcpStream, port: u16) {
    match TcpStream::from_std(local) {
        Ok(local) => carry_both(channel, local, port).await,
        // The channel is open on the host; close it rather than leave it
        // until the session ends.
        Err(_) => {
            let _ = channel.close().await;
        }
    }
}

/// How a carried connection ended.
enum Ended {
    /// Both directions finished.
    Both,
    /// The caller left more than [`UNREAD_LIMIT`] bytes unread.
    Behind,
    /// The caller's connection can no longer be written.
    CallerGone,
}

/// Copies both ways between the caller's connection and the channel until
/// both have ended. EOF from either side reaches the other as EOF. A caller
/// that falls [`UNREAD_LIMIT`] behind is reset; one that goes away ends the
/// connection.
async fn carry_both(channel: Channel<Msg>, local: TcpStream, port: u16) {
    let _ = local.set_nodelay(true);
    let (mut remote_read, remote_write) = channel.split();
    let mut writer = remote_write.make_writer();
    let (local_read, mut local_write) = local.into_split();
    let ended = {
        let upstream = async move {
            let mut local_read = BufReader::with_capacity(RELAY_BUFFER, local_read);
            // An error means the channel is gone, which the downstream sees.
            if copy_buf(&mut local_read, &mut writer).await.is_ok() {
                let _ = writer.shutdown().await;
            }
        };
        let downstream = downstream(&mut remote_read, &mut local_write);
        tokio::pin!(upstream, downstream);
        let (mut up, mut down) = (false, false);
        loop {
            if up && down {
                break Ended::Both;
            }
            tokio::select! {
                () = &mut upstream, if !up => up = true,
                ended = &mut downstream, if !down => match ended {
                    Downstream::Ended => down = true,
                    Downstream::Behind => break Ended::Behind,
                    Downstream::CallerGone => break Ended::CallerGone,
                },
            }
        }
    };
    // Nothing reads the channel from here: russh then drops what arrives
    // for it instead of waiting on its queue, so the close below cannot
    // wait behind a session task stuck on this channel.
    drop(remote_read);
    if let Ended::Behind = ended {
        tracing::warn!(
            category = "ssh",
            port,
            limit = UNREAD_LIMIT,
            "forwarded connection reset: its reader fell behind"
        );
        // A reset, not EOF, so the caller cannot take a cut-off response
        // for a whole one: linger zero, and the write half is forgotten
        // rather than shut down, so no FIN goes first.
        let _ = local_write.as_ref().set_zero_linger();
        local_write.forget();
    }
    // Closes the host's end on every path, as `ChannelStream`'s drop does.
    let _ = remote_write.close().await;
}

enum Downstream {
    /// The channel ended and the caller has everything it sent.
    Ended,
    /// The caller left more than [`UNREAD_LIMIT`] bytes unread.
    Behind,
    /// The caller's connection can no longer be written.
    CallerGone,
}

/// Moves the channel's data to the caller. The channel is read whenever it
/// has something, whether or not the caller is reading, so the session task
/// is never left waiting on this channel's queue.
async fn downstream(
    remote: &mut ChannelReadHalf,
    local: &mut (impl AsyncWrite + Unpin),
) -> Downstream {
    let mut unread = VecDeque::new();
    // Bytes of `unread`'s first chunk already written.
    let mut written = 0;
    let mut queued = 0;
    let mut ended = false;
    loop {
        if ended && unread.is_empty() {
            let _ = local.shutdown().await;
            return Downstream::Ended;
        }
        let step = {
            let pending = unread.front().map_or(&[][..], |chunk| rest(chunk, written));
            tokio::select! {
                message = remote.wait(), if !ended => Step::Received(message),
                wrote = local.write(pending), if !pending.is_empty() => Step::Wrote(wrote),
            }
        };
        match step {
            Step::Received(Some(ChannelMsg::Data { data })) => {
                queued += data.len();
                if queued > UNREAD_LIMIT {
                    return Downstream::Behind;
                }
                if !data.is_empty() {
                    unread.push_back(data);
                }
            }
            // The session's end closes every channel's queue.
            Step::Received(Some(ChannelMsg::Eof | ChannelMsg::Close) | None) => ended = true,
            Step::Received(Some(_)) => {}
            Step::Wrote(Ok(0) | Err(_)) => return Downstream::CallerGone,
            Step::Wrote(Ok(count)) => {
                written += count;
                queued -= count;
                if unread.front().is_some_and(|chunk| written == chunk.len()) {
                    unread.pop_front();
                    written = 0;
                }
            }
        }
    }
}

/// What of `chunk` is left after `from` bytes. Generic so the chunk type,
/// russh's, need not be named here.
fn rest<T: AsRef<[u8]>>(chunk: &T, from: usize) -> &[u8] {
    chunk.as_ref().get(from..).unwrap_or_default()
}

enum Step {
    Received(Option<ChannelMsg>),
    Wrote(std::io::Result<usize>),
}
