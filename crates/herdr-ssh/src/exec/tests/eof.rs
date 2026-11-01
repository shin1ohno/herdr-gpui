#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::{
    io::{ErrorKind, Read as _},
    net::TcpListener,
    sync::Arc,
    time::Duration,
};

use russh::{
    Channel, ChannelId,
    keys::{PrivateKey, ssh_key::private::Ed25519Keypair},
    server,
};
use tokio::sync::oneshot;

use crate::{ClientKey, HostKey, Pins, Target, TunnelExit, runtime};

struct Peer {
    finish: Option<oneshot::Receiver<()>>,
}

impl server::Handler for Peer {
    type Error = russh::Error;

    async fn auth_publickey(
        &mut self,
        _user: &str,
        _key: server::VerifiedPublicKey<'_>,
    ) -> Result<server::Auth, Self::Error> {
        Ok(server::Auth::Accept)
    }

    async fn channel_open_session(
        &mut self,
        _channel: Channel<server::Msg>,
        reply: server::ChannelOpenHandle,
        _session: &mut server::Session,
    ) -> Result<(), Self::Error> {
        reply.accept().await;
        Ok(())
    }

    async fn exec_request(
        &mut self,
        channel: ChannelId,
        _command: &[u8],
        session: &mut server::Session,
    ) -> Result<(), Self::Error> {
        session.channel_success(channel)?;
        session.data(channel, b"ok".as_slice())?;
        session.eof(channel)?;
        let finish = self.finish.take().unwrap();
        let handle = session.handle();
        tokio::spawn(async move {
            finish.await.unwrap();
            handle.exit_status_request(channel, 37).await.unwrap();
            handle.close(channel).await.unwrap();
        });
        Ok(())
    }
}

/// A peer holds exit-status after sending EOF, exposing the race without
/// depending on how fast OpenSSH normally sends the two messages.
#[test]
fn remote_eof_waits_for_the_recorded_exit() {
    let key = PrivateKey::new(Ed25519Keypair::from_seed(&[19; 32]).into(), "test-only").unwrap();
    let pin = HostKey::from_blob(key.public_key().to_bytes().unwrap()).unwrap();
    let server_config = Arc::new(server::Config {
        keys: vec![key],
        ..server::Config::default()
    });
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    listener.set_nonblocking(true).unwrap();
    let runtime = runtime::handle().unwrap();
    let listener = {
        let _entered = runtime.enter();
        tokio::net::TcpListener::from_std(listener).unwrap()
    };
    let (finish, gate) = oneshot::channel();
    let peer = super::super::AbortOnDrop(runtime.spawn(async move {
        let (socket, _) = listener.accept().await.unwrap();
        let session = server::run_stream(server_config, socket, Peer { finish: Some(gate) })
            .await
            .unwrap();
        let _ = session.await;
    }));
    let target = Target::parse(&format!("test@127.0.0.1:{port}")).unwrap();
    let client: ClientKey = crate::client_key::tests::software_key(3);
    let (mut stream, tunnel) =
        crate::exec(&target, &client, &Pins::Keys(vec![pin]), "ignored").unwrap();
    stream
        .set_read_timeout(Some(Duration::from_secs(10)))
        .unwrap();
    let mut prefix = [0; 2];
    stream.read_exact(&mut prefix).unwrap();
    assert_eq!(&prefix, b"ok");
    stream
        .set_read_timeout(Some(Duration::from_millis(200)))
        .unwrap();
    let error = stream
        .read(&mut [0])
        .expect_err("EOF must wait for the remote exit");
    assert!(matches!(
        error.kind(),
        ErrorKind::WouldBlock | ErrorKind::TimedOut
    ));
    assert_eq!(tunnel.wait_exit(Duration::ZERO), None);
    finish.send(()).unwrap();
    stream
        .set_read_timeout(Some(Duration::from_secs(10)))
        .unwrap();
    assert_eq!(stream.read_to_end(&mut Vec::new()).unwrap(), 0);
    assert_eq!(
        tunnel.wait_exit(Duration::ZERO),
        Some(TunnelExit::Remote {
            exit_status: Some(37),
            stderr_tail: Vec::new()
        })
    );
    drop(tunnel);
    drop(peer);
}
