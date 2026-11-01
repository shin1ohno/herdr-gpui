//! Bounded stdout from an in-process exec channel. A timeout is not EOF, and
//! cancellation discards the incomplete reply instead of accepting a prefix.
use crate::Result;
use std::{
    io::{self, Read},
    time::Instant,
};

pub(super) fn read(
    stream: &mut impl Read,
    deadline: Instant,
    output_limit: u64,
    cancelled: impl Fn() -> bool,
) -> Result<Vec<u8>> {
    let output_limit = output_limit.min(super::PROBE_OUTPUT_LIMIT);
    let mut output = Vec::new();
    let mut buffer = [0u8; 4096];
    loop {
        if cancelled() {
            return Err(super::Error::Cancelled.into());
        }
        if Instant::now() >= deadline {
            return Err(super::Error::Timeout.into());
        }
        let remaining = output_limit.saturating_sub(output.len() as u64);
        let reading = remaining.saturating_add(1).min(buffer.len() as u64) as usize;
        match stream.read(&mut buffer[..reading]) {
            Ok(0) => return Ok(output),
            Ok(n) => {
                if n as u64 > remaining {
                    return Err(super::Error::OutputLimit.into());
                }
                output.extend_from_slice(&buffer[..n]);
            }
            Err(error)
                if matches!(
                    error.kind(),
                    io::ErrorKind::WouldBlock
                        | io::ErrorKind::TimedOut
                        | io::ErrorKind::Interrupted
                ) => {}
            Err(error) => return Err(error.into()),
        }
    }
}

#[cfg(test)]
mod tests;
