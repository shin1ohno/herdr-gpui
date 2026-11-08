use std::io;

/// Errors from Apple's DNS-SD service, retaining its numeric status.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    // keep-sorted start
    #[error("Searching the local network failed while {operation}: {status}")]
    DnsSd {
        operation: &'static str,
        #[source]
        status: Status,
    },
    #[error("Waiting for Bonjour failed: {0}")]
    Poll(#[source] io::Error),
    // keep-sorted end
}

#[derive(Debug, thiserror::Error)]
#[error("DNS-SD status {0}")]
pub struct Status(pub(super) i32);

impl Status {
    /// Apple's numeric DNS-SD status.
    pub(crate) fn code(&self) -> i32 {
        self.0
    }
}

impl Error {
    pub(super) fn status(operation: &'static str, code: i32) -> Self {
        Self::DnsSd {
            operation,
            status: Status(code),
        }
    }

    pub(super) fn denied(&self) -> bool {
        matches!(
            self,
            Self::DnsSd {
                status: Status(-65570),
                ..
            }
        )
    }
}
