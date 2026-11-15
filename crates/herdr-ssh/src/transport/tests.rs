#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::time::Duration;

use tokio::io::{AsyncReadExt as _, AsyncWriteExt as _};

use super::*;

fn block_on<F: Future>(future: F) -> F::Output {
    tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .build()
        .unwrap()
        .block_on(future)
}

#[test]
fn kill_wakes_a_pending_read_and_fails_every_later_call() {
    block_on(async {
        // The peer half stays open and silent, so the read would wait forever.
        let (near, _far) = tokio::io::duplex(64);
        let switch = Arc::new(KillSwitch::default());
        let mut stream = Killable::new(near, Arc::clone(&switch));
        let killer = {
            let switch = Arc::clone(&switch);
            tokio::spawn(async move {
                tokio::time::sleep(Duration::from_millis(20)).await;
                switch.kill();
            })
        };
        let read = tokio::time::timeout(Duration::from_secs(5), stream.read(&mut [0; 8]))
            .await
            .expect("kill must wake the read");
        assert_eq!(read.unwrap_err().kind(), io::ErrorKind::ConnectionAborted);
        assert_eq!(
            stream.write_all(b"x").await.unwrap_err().kind(),
            io::ErrorKind::ConnectionAborted
        );
        assert_eq!(
            stream.shutdown().await.unwrap_err().kind(),
            io::ErrorKind::ConnectionAborted
        );
        killer.await.unwrap();
        assert!(!switch.released());
        drop(stream);
        assert!(switch.released());
    });
}

#[test]
fn an_uncut_stream_passes_bytes_through() {
    block_on(async {
        let (near, mut far) = tokio::io::duplex(64);
        let switch = Arc::new(KillSwitch::default());
        let mut stream = Killable::new(near, Arc::clone(&switch));
        stream.write_all(b"ping").await.unwrap();
        let mut got = [0; 4];
        far.read_exact(&mut got).await.unwrap();
        assert_eq!(&got, b"ping");
        far.write_all(b"pong").await.unwrap();
        stream.read_exact(&mut got).await.unwrap();
        assert_eq!(&got, b"pong");
    });
}

#[test]
fn wait_released_resolves_when_the_stream_drops() {
    block_on(async {
        let (near, _far) = tokio::io::duplex(64);
        let switch = Arc::new(KillSwitch::default());
        let stream = Killable::new(near, Arc::clone(&switch));
        let short = Duration::from_millis(20);
        assert!(
            tokio::time::timeout(short, switch.wait_released())
                .await
                .is_err()
        );
        let dropper = tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(20)).await;
            drop(stream);
        });
        assert!(
            tokio::time::timeout(Duration::from_secs(5), switch.wait_released())
                .await
                .is_ok()
        );
        dropper.await.unwrap();
    });
}
