use thiserror::Error;

use crate::Failure;

/// Why a forward could not carry a connection. Every variant is classified
/// from russh's typed results or this crate's own checks; nothing the host
/// wrote (an open failure's description, a banner) reaches it.
#[derive(Debug, Error)]
pub enum Error {
    // keep-sorted start
    /// The forward already carries [`MAX_CHANNELS`](super::MAX_CHANNELS)
    /// connections or has that many requests waiting.
    #[error("too many forwarded connections at once")]
    Busy,
    /// The forwarding connection could not be set up: the host was
    /// unreachable, its key is not pinned or changed, or it refused this
    /// device's key.
    #[error("the forwarding connection failed: {0}")]
    Connect(#[source] Failure),
    /// The forwarding connection was set up and has since ended: the host
    /// or the network dropped it, or it stopped answering keepalives.
    #[error("the forwarding connection was lost: {0}")]
    ConnectionLost(#[source] Failure),
    /// A port that is not 1 to 65535.
    #[error("invalid port to forward to")]
    InvalidPort,
    #[error(transparent)]
    Io(#[from] std::io::Error),
    /// A destination host other than the host's own loopback.
    #[error("only the host's loopback can be forwarded to")]
    NotLoopback,
    /// The host accepted the channel request but nothing accepted the TCP
    /// connection on that port (`SSH_OPEN_CONNECT_FAILED`).
    #[error("nothing is listening on that port on the host")]
    PortClosed,
    /// The host does not allow this key to forward there
    /// (`SSH_OPEN_ADMINISTRATIVELY_PROHIBITED`): a `restrict` or
    /// `permitopen` key option, or `AllowTcpForwarding no`.
    #[error("the host does not allow forwarding for this key")]
    Prohibited,
    /// The host refused the channel for another reason: an unknown channel
    /// type, a resource shortage, or a reason code outside RFC 4254.
    #[error("the host refused the forwarded connection")]
    Refused,
    /// The forward was stopped or dropped by its owner.
    #[error("the forward was stopped")]
    Stopped,
    /// The host did not answer the channel request in time.
    #[error("the host did not answer the forwarded connection in time")]
    Timeout,
    // keep-sorted end
}

pub type Result<T, E = Error> = std::result::Result<T, E>;
