#![allow(clippy::unwrap_used)]
use super::*;
use std::{
    collections::VecDeque,
    io::{Cursor, ErrorKind},
    time::Duration,
};

fn deadline() -> Instant {
    Instant::now() + Duration::from_secs(60)
}

const OUTPUT_LIMIT: u64 = super::super::PROBE_OUTPUT_LIMIT;

struct Fragmented(VecDeque<std::result::Result<&'static [u8], ErrorKind>>);

impl Read for Fragmented {
    fn read(&mut self, output: &mut [u8]) -> io::Result<usize> {
        match self.0.pop_front() {
            Some(Ok(bytes)) => {
                output[..bytes.len()].copy_from_slice(bytes);
                Ok(bytes.len())
            }
            Some(Err(kind)) => Err(kind.into()),
            None => Ok(0),
        }
    }
}

#[test]
fn fragments_and_transient_timeouts_do_not_truncate_a_reply() {
    let mut input = Fragmented(VecDeque::from([
        Ok(&b"{"[..]),
        Err(ErrorKind::TimedOut),
        Ok(&b"reply"[..]),
        Err(ErrorKind::Interrupted),
        Err(ErrorKind::WouldBlock),
        Ok(&b"}"[..]),
    ]));
    assert_eq!(
        read(&mut input, deadline(), OUTPUT_LIMIT, || false).unwrap(),
        b"{reply}"
    );
}

#[test]
fn exact_output_limit_is_accepted_and_an_extra_byte_is_refused() {
    let limit = super::super::PROBE_OUTPUT_LIMIT as usize;
    let mut input = Cursor::new(vec![b'x'; limit]);
    assert_eq!(
        read(&mut input, deadline(), OUTPUT_LIMIT, || false)
            .unwrap()
            .len(),
        limit
    );
    let mut input = Cursor::new(vec![b'x'; limit + 1]);
    assert!(matches!(
        read(&mut input, deadline(), OUTPUT_LIMIT, || false),
        Err(crate::Error::Ssh(super::super::Error::OutputLimit))
    ));
}

#[test]
fn cancellation_or_an_expired_deadline_precedes_reading() {
    let mut input = Cursor::new(b"must not be read");
    assert!(matches!(
        read(&mut input, deadline(), OUTPUT_LIMIT, || true),
        Err(crate::Error::Ssh(super::super::Error::Cancelled))
    ));
    assert_eq!(input.position(), 0);
    assert!(matches!(
        read(
            &mut input,
            Instant::now() - Duration::from_secs(1),
            OUTPUT_LIMIT,
            || false
        ),
        Err(crate::Error::Ssh(super::super::Error::Timeout))
    ));
    assert_eq!(input.position(), 0);
}

#[test]
fn broken_streams_retain_the_original_io_error() {
    let mut input = Fragmented(VecDeque::from([Err(ErrorKind::ConnectionReset)]));
    assert!(
        matches!(read(&mut input, deadline(), OUTPUT_LIMIT, || false),
        Err(crate::Error::Io(error)) if error.kind() == ErrorKind::ConnectionReset)
    );
}

#[test]
fn callers_can_lower_the_limit_and_excess_output_stops_after_one_extra_byte() {
    let mut input = Cursor::new(b"abc");
    assert_eq!(read(&mut input, deadline(), 3, || false).unwrap(), b"abc");
    let mut input = Cursor::new(b"abcdmust not be read");
    assert!(matches!(
        read(&mut input, deadline(), 3, || false),
        Err(crate::Error::Ssh(super::super::Error::OutputLimit))
    ));
    assert_eq!(input.position(), 4);
    let mut input = Cursor::new(vec![b'x'; OUTPUT_LIMIT as usize + 2]);
    assert!(matches!(
        read(&mut input, deadline(), u64::MAX, || false),
        Err(crate::Error::Ssh(super::super::Error::OutputLimit))
    ));
    assert_eq!(input.position(), OUTPUT_LIMIT + 1);
}
