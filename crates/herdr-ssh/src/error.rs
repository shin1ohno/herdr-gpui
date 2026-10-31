use thiserror::Error;

/// Why an SSH connection failed, classified from russh's typed results and
/// never from text the server sent. `herdr-client` maps each class onto its
/// own `SshFailure`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum Failure {
    /// The host offered a key and no key is pinned for it.
    #[error("the host key is not trusted yet")]
    HostKeyUnknown,
    /// A key is pinned for the host and the offered key matches none of the
    /// pinned keys, whatever its type; also no common host key algorithm with
    /// the pinned types, a bad server signature, or a host certificate.
    #[error("the host key changed")]
    HostKeyChanged,
    /// The client key was refused, including partial success.
    #[error("the host refused this device's key")]
    Auth,
    /// Name resolution, TCP connect, or the connection dropped.
    #[error("the host is unreachable")]
    Unreachable,
    /// Connect, key exchange, or keepalive ran past its deadline.
    #[error("the host did not answer in time")]
    Timeout,
    /// The peer broke the protocol or offered nothing in the allow-list.
    #[error("the SSH connection failed")]
    Protocol,
    /// The caller asked to stop.
    #[error("cancelled")]
    Cancelled,
}

/// Errors from this crate's API. Connection failures are [`Failure`]s;
/// the other variants are local.
#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    Failure(#[from] Failure),
    /// A target that is not `user@host[:port]` as docs/ios.md defines it.
    #[error("invalid SSH target: {reason}")]
    InvalidTarget { reason: &'static str },
    /// The `known_hosts` file could not be read, parsed, or written.
    #[error("known hosts file {operation}")]
    KnownHosts {
        operation: &'static str,
        #[source]
        source: std::io::Error,
    },
    /// The host gained a pin between the scan and the trust.
    #[error("the host gained a trusted key before approval")]
    PinnedMeanwhile,
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

pub type Result<T, E = Error> = std::result::Result<T, E>;
