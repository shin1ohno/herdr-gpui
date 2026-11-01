use thiserror::Error;

/// Why an SSH connection failed, classified from russh's typed results and
/// never from text the server sent. `herdr-client` maps each class onto its
/// own `SshFailure`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum Failure {
    // keep-sorted start
    /// The client key was refused, including partial success.
    #[error("the host refused this device's key")]
    Auth,
    /// The caller asked to stop.
    #[error("cancelled")]
    Cancelled,
    /// A key is pinned for the host and the offered key matches none of the
    /// pinned keys, whatever its type; also no common host key algorithm with
    /// the pinned types, a bad server signature, or a host certificate.
    #[error("the host key changed")]
    HostKeyChanged,
    /// The host offered a key and no key is pinned for it.
    #[error("the host key is not trusted yet")]
    HostKeyUnknown,
    /// The peer broke the protocol or offered nothing in the allow-list.
    #[error("the SSH connection failed")]
    Protocol,
    /// Connect, key exchange, or keepalive ran past its deadline.
    #[error("the host did not answer in time")]
    Timeout,
    /// Name resolution, TCP connect, or the connection dropped.
    #[error("the host is unreachable")]
    Unreachable,
    // keep-sorted end
}

/// Errors from this crate's API. Connection failures are [`Failure`]s;
/// the other variants are local.
#[derive(Debug, Error)]
pub enum Error {
    // keep-sorted start
    #[error(transparent)]
    Failure(#[from] Failure),
    /// A client public key that is not an uncompressed P-256 point.
    #[error("invalid device key: {reason}")]
    InvalidClientKey { reason: &'static str },
    /// A target that is not `user@host[:port]` as docs/ios.md defines it.
    #[error("invalid SSH target: {reason}")]
    InvalidTarget { reason: &'static str },
    #[error(transparent)]
    Io(#[from] std::io::Error),
    /// The `known_hosts` file could not be read, parsed, or written.
    #[error("known hosts file {operation}")]
    KnownHosts {
        operation: &'static str,
        #[source]
        source: std::io::Error,
    },
    /// The host's pins changed between the review and its confirmation.
    #[error("the host's trusted keys changed before confirmation")]
    PinnedMeanwhile,
    // keep-sorted end
}

pub type Result<T, E = Error> = std::result::Result<T, E>;
