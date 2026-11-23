//! Hosts for this crate's tests, on 127.0.0.1 ephemeral ports: an
//! in-process russh server with a host key generated from a fixed test seed,
//! and raw TCP peers that stop part-way through the handshake. Nothing here
//! starts an external `sshd` or reads a real key.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::{
    io::{ErrorKind, Read as _, Write as _},
    net::{TcpListener, TcpStream},
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
    thread,
    time::{Duration, Instant},
};

use russh::{
    Channel, ChannelId,
    keys::{PrivateKey, ssh_key::private::Ed25519Keypair},
    server,
};
use tokio::sync::{Semaphore, oneshot};

use crate::{HostKey, Pins};

/// What the server does once it receives the exec request.
#[derive(Debug, Clone)]
pub(crate) enum Behavior {
    /// Writes `stdout`, then EOF, exit status, and Close.
    Exit { stdout: Vec<u8>, status: u32 },
    /// Writes standard output until the channel goes away.
    Flood,
    /// Accepts the request and then says nothing.
    Silent,
    /// Ends the SSH session with a disconnect message right after accepting
    /// the exec request; the channel gets no exit status and no Close.
    DisconnectAfterExec,
    /// Drops the connection right after accepting the exec request, as a
    /// host that crashed or a network that went away.
    HangUpAfterExec,
    /// Drops the connection while answering the authentication request.
    HangUpAtAuth,
    /// Opens a forwarded-tcpip, an X11, an agent, and a session channel
    /// back to the client after each `direct-tcpip` channel it accepts, and
    /// counts the ones the client refuses.
    OpensChannelsBack,
    /// Refuses every `direct-tcpip` channel as administratively prohibited,
    /// as OpenSSH does for a `restrict` key.
    ProhibitForwarding,
}

/// Counts of what the server saw, shared with the test.
#[derive(Debug, Default)]
pub(crate) struct Seen {
    pub(crate) connections: AtomicUsize,
    /// Signed public-key requests that reached the server's check.
    pub(crate) auths: AtomicUsize,
    pub(crate) execs: AtomicUsize,
    /// Connections whose SSH session has ended on the server's side.
    pub(crate) ended: AtomicUsize,
    /// Session channels the client opened.
    pub(crate) sessions: AtomicUsize,
    /// `direct-tcpip` channels the client asked for, accepted or not.
    pub(crate) direct_tcpip: AtomicUsize,
    /// Remote, Unix-socket, X11, or agent forwarding the client asked for.
    pub(crate) forwarding_requests: AtomicUsize,
    /// Channels the server opened that the client refused.
    pub(crate) refused_by_client: AtomicUsize,
}

impl Seen {
    pub(crate) fn get(counter: &AtomicUsize) -> usize {
        counter.load(Ordering::SeqCst)
    }
}

pub(crate) struct TestHost {
    pub(crate) port: u16,
    pub(crate) pins: Pins,
    pub(crate) seen: Arc<Seen>,
    /// Permits for authentication replies; `None` answers at once.
    auth_gate: Option<Arc<Semaphore>>,
    stop: Option<oneshot::Sender<()>>,
    thread: Option<thread::JoinHandle<()>>,
}

impl TestHost {
    pub(crate) fn start(behavior: Behavior) -> Self {
        Self::start_with(behavior, false)
    }

    /// A host that holds each authentication reply until
    /// [`TestHost::answer_auth`] lets one through.
    pub(crate) fn holding_auth(behavior: Behavior) -> Self {
        Self::start_with(behavior, true)
    }

    fn start_with(behavior: Behavior, hold_auth: bool) -> Self {
        let key =
            PrivateKey::new(Ed25519Keypair::from_seed(&[23; 32]).into(), "test-only").unwrap();
        let pin = HostKey::from_blob(key.public_key().to_bytes().unwrap()).unwrap();
        let config = Arc::new(server::Config {
            keys: vec![key],
            ..server::Config::default()
        });
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let port = listener.local_addr().unwrap().port();
        let seen = Arc::new(Seen::default());
        let auth_gate = hold_auth.then(|| Arc::new(Semaphore::new(0)));
        let (stop, stopped) = oneshot::channel();
        let thread = {
            let seen = Arc::clone(&seen);
            let auth_gate = auth_gate.clone();
            thread::spawn(move || serve(listener, config, behavior, seen, auth_gate, stopped))
        };
        Self {
            port,
            pins: Pins::Keys(vec![pin]),
            seen,
            auth_gate,
            stop: Some(stop),
            thread: Some(thread),
        }
    }

    pub(crate) fn answer_auth(&self) {
        if let Some(gate) = &self.auth_gate {
            gate.add_permits(1);
        }
    }

    /// Waits until `counter` reaches `at_least`, at most `limit`.
    pub(crate) fn wait_for(counter: &AtomicUsize, at_least: usize, limit: Duration) -> bool {
        let started = Instant::now();
        while Seen::get(counter) < at_least {
            if started.elapsed() > limit {
                return false;
            }
            thread::sleep(Duration::from_millis(5));
        }
        true
    }
}

impl Drop for TestHost {
    fn drop(&mut self) {
        if let Some(stop) = self.stop.take() {
            let _ = stop.send(());
        }
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

fn serve(
    listener: TcpListener,
    config: Arc<server::Config>,
    behavior: Behavior,
    seen: Arc<Seen>,
    auth_gate: Option<Arc<Semaphore>>,
    mut stopped: oneshot::Receiver<()>,
) {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    runtime.block_on(async move {
        let listener = tokio::net::TcpListener::from_std(listener).unwrap();
        loop {
            let socket = tokio::select! {
                _ = &mut stopped => return,
                accepted = listener.accept() => match accepted {
                    Ok((socket, _)) => socket,
                    Err(_) => continue,
                },
            };
            seen.connections.fetch_add(1, Ordering::SeqCst);
            let peer = Peer {
                behavior: behavior.clone(),
                seen: Arc::clone(&seen),
                auth_gate: auth_gate.clone(),
            };
            let config = Arc::clone(&config);
            let seen = Arc::clone(&seen);
            tokio::spawn(async move {
                if let Ok(session) = server::run_stream(config, socket, peer).await {
                    let _ = session.await;
                }
                seen.ended.fetch_add(1, Ordering::SeqCst);
            });
        }
    });
}

struct Peer {
    behavior: Behavior,
    seen: Arc<Seen>,
    auth_gate: Option<Arc<Semaphore>>,
}

impl server::Handler for Peer {
    type Error = russh::Error;

    async fn auth_publickey(
        &mut self,
        _user: &str,
        _key: server::VerifiedPublicKey<'_>,
    ) -> Result<server::Auth, Self::Error> {
        self.seen.auths.fetch_add(1, Ordering::SeqCst);
        if matches!(self.behavior, Behavior::HangUpAtAuth) {
            // A handler error ends the server's session and its connection.
            return Err(russh::Error::Disconnect);
        }
        if let Some(gate) = &self.auth_gate
            && let Ok(permit) = gate.acquire().await
        {
            permit.forget();
        }
        Ok(server::Auth::Accept)
    }

    async fn channel_open_session(
        &mut self,
        _channel: Channel<server::Msg>,
        reply: server::ChannelOpenHandle,
        _session: &mut server::Session,
    ) -> Result<(), Self::Error> {
        self.seen.sessions.fetch_add(1, Ordering::SeqCst);
        reply.accept().await;
        Ok(())
    }

    /// Connects to `port_to_connect` on this machine's loopback and relays,
    /// as sshd does; a closed port is refused as `ConnectFailed`.
    async fn channel_open_direct_tcpip(
        &mut self,
        channel: Channel<server::Msg>,
        host_to_connect: &str,
        port_to_connect: u32,
        _originator_address: &str,
        _originator_port: u32,
        reply: server::ChannelOpenHandle,
        session: &mut server::Session,
    ) -> Result<(), Self::Error> {
        self.seen.direct_tcpip.fetch_add(1, Ordering::SeqCst);
        if matches!(self.behavior, Behavior::ProhibitForwarding) || host_to_connect != "localhost" {
            reply
                .reject(russh::ChannelOpenFailure::AdministrativelyProhibited)
                .await;
            return Ok(());
        }
        let Ok(port) = u16::try_from(port_to_connect) else {
            reply.reject(russh::ChannelOpenFailure::ConnectFailed).await;
            return Ok(());
        };
        let Ok(mut socket) = tokio::net::TcpStream::connect(("127.0.0.1", port)).await else {
            reply.reject(russh::ChannelOpenFailure::ConnectFailed).await;
            return Ok(());
        };
        reply.accept().await;
        tokio::spawn(async move {
            let mut channel = channel.into_stream();
            let _ = tokio::io::copy_bidirectional(&mut socket, &mut channel).await;
        });
        if matches!(self.behavior, Behavior::OpensChannelsBack) {
            let handle = session.handle();
            let seen = Arc::clone(&self.seen);
            tokio::spawn(async move {
                let refused = [
                    handle
                        .channel_open_forwarded_tcpip("localhost", 80, "127.0.0.1", 1)
                        .await
                        .is_err(),
                    handle.channel_open_x11("127.0.0.1", 1).await.is_err(),
                    handle.channel_open_agent().await.is_err(),
                    handle.channel_open_session().await.is_err(),
                ];
                let refused = refused.into_iter().filter(|refused| *refused).count();
                seen.refused_by_client.fetch_add(refused, Ordering::SeqCst);
            });
        }
        Ok(())
    }

    async fn tcpip_forward(
        &mut self,
        _address: &str,
        _port: &mut u32,
        _session: &mut server::Session,
    ) -> Result<bool, Self::Error> {
        self.seen.forwarding_requests.fetch_add(1, Ordering::SeqCst);
        Ok(false)
    }

    async fn streamlocal_forward(
        &mut self,
        _socket_path: &str,
        _session: &mut server::Session,
    ) -> Result<bool, Self::Error> {
        self.seen.forwarding_requests.fetch_add(1, Ordering::SeqCst);
        Ok(false)
    }

    async fn x11_request(
        &mut self,
        _channel: ChannelId,
        _single_connection: bool,
        _x11_auth_protocol: &str,
        _x11_auth_cookie: &str,
        _x11_screen_number: u32,
        _session: &mut server::Session,
    ) -> Result<(), Self::Error> {
        self.seen.forwarding_requests.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }

    async fn agent_request(
        &mut self,
        _channel: ChannelId,
        _session: &mut server::Session,
    ) -> Result<bool, Self::Error> {
        self.seen.forwarding_requests.fetch_add(1, Ordering::SeqCst);
        Ok(false)
    }

    async fn exec_request(
        &mut self,
        channel: ChannelId,
        _command: &[u8],
        session: &mut server::Session,
    ) -> Result<(), Self::Error> {
        self.seen.execs.fetch_add(1, Ordering::SeqCst);
        session.channel_success(channel)?;
        let handle = session.handle();
        match self.behavior.clone() {
            Behavior::Exit { stdout, status } => {
                tokio::spawn(async move {
                    let _ = handle.data(channel, stdout).await;
                    let _ = handle.eof(channel).await;
                    let _ = handle.exit_status_request(channel, status).await;
                    let _ = handle.close(channel).await;
                });
            }
            Behavior::Flood => {
                tokio::spawn(async move {
                    let chunk = vec![b'x'; 8192];
                    while handle.data(channel, chunk.clone()).await.is_ok() {}
                });
            }
            Behavior::Silent
            | Behavior::HangUpAtAuth
            | Behavior::OpensChannelsBack
            | Behavior::ProhibitForwarding => {}
            Behavior::DisconnectAfterExec => {
                session.disconnect(russh::Disconnect::ByApplication, "", "")?;
            }
            Behavior::HangUpAfterExec => return Err(russh::Error::Disconnect),
        }
        Ok(())
    }
}

/// A TCP peer that accepts and then stops part-way through the SSH
/// handshake: before its identification line, or right after it.
pub(crate) struct StalledPeer {
    pub(crate) port: u16,
    /// Connections accepted.
    pub(crate) accepted: Arc<AtomicUsize>,
    /// Accepted connections the client has since closed.
    pub(crate) closed: Arc<AtomicUsize>,
    stop: Arc<AtomicBool>,
    thread: Option<thread::JoinHandle<()>>,
}

impl StalledPeer {
    /// `identify` sends `SSH-2.0-…` first, so the client goes on to the key
    /// exchange and waits there.
    pub(crate) fn start(identify: bool) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let port = listener.local_addr().unwrap().port();
        let accepted = Arc::new(AtomicUsize::new(0));
        let closed = Arc::new(AtomicUsize::new(0));
        let stop = Arc::new(AtomicBool::new(false));
        let thread = {
            let (accepted, closed, stop) = (
                Arc::clone(&accepted),
                Arc::clone(&closed),
                Arc::clone(&stop),
            );
            thread::spawn(move || stall(listener, identify, &accepted, &closed, &stop))
        };
        Self {
            port,
            accepted,
            closed,
            stop,
            thread: Some(thread),
        }
    }
}

impl Drop for StalledPeer {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

/// Holds every accepted socket open, reading and discarding what the client
/// sends, and counts the ones the client closes.
fn stall(
    listener: TcpListener,
    identify: bool,
    accepted: &AtomicUsize,
    closed: &AtomicUsize,
    stop: &AtomicBool,
) {
    let mut open: Vec<TcpStream> = Vec::new();
    while !stop.load(Ordering::SeqCst) {
        if let Ok((mut socket, _)) = listener.accept() {
            socket.set_nonblocking(true).unwrap();
            if identify {
                let _ = socket.write_all(b"SSH-2.0-stalled\r\n");
            }
            accepted.fetch_add(1, Ordering::SeqCst);
            open.push(socket);
        }
        open.retain_mut(|socket| {
            let mut buffer = [0; 4096];
            match socket.read(&mut buffer) {
                Ok(0) => {
                    closed.fetch_add(1, Ordering::SeqCst);
                    false
                }
                Ok(_) => true,
                Err(error) if error.kind() == ErrorKind::WouldBlock => true,
                Err(_) => {
                    closed.fetch_add(1, Ordering::SeqCst);
                    false
                }
            }
        });
        thread::sleep(Duration::from_millis(5));
    }
}
