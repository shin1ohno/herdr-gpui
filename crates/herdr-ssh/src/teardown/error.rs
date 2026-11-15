use thiserror::Error;

/// Why a tunnel could not be given a teardown identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum Error {
    /// The process has started 2^64 - 1 tunnels; identities never repeat, so
    /// no more can start.
    #[error("no tunnel identities are left in this process")]
    IdsExhausted,
}
