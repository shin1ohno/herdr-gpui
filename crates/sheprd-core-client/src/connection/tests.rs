#![allow(clippy::unwrap_used)]

use super::*;

struct InterruptedWrite {
    stop: Arc<AtomicBool>,
    written: Vec<Vec<u8>>,
    flushes: usize,
}

impl Read for InterruptedWrite {
    fn read(&mut self, _: &mut [u8]) -> io::Result<usize> {
        Err(io::ErrorKind::WouldBlock.into())
    }
}

impl Write for InterruptedWrite {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.written.push(bytes.to_vec());
        self.stop.store(true, Ordering::Release);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        self.flushes += 1;
        Ok(())
    }
}

#[test]
fn cancellation_does_not_drain_or_flush_the_rest_of_a_write_batch() {
    let stop = Arc::new(AtomicBool::new(false));
    let (send, queue) = crossbeam_channel::bounded(3);
    send.send(Outbound {
        bytes: b"first".to_vec(),
        gate: None,
    })
    .unwrap();
    send.send(Outbound {
        bytes: b"queued".to_vec(),
        gate: None,
    })
    .unwrap();
    let mut worker = Worker {
        decoder: Decoder::default(),
        queue,
        shared: Arc::new(Shared::default()),
        stop: Arc::clone(&stop),
        stream: InterruptedWrite {
            stop,
            written: Vec::new(),
            flushes: 0,
        },
    };
    assert!(!worker.write_queued().unwrap());
    assert_eq!(worker.stream.written, [b"first".to_vec()]);
    assert_eq!(worker.stream.flushes, 0);
    assert_eq!(worker.queue.try_recv().unwrap().bytes, b"queued");
}
