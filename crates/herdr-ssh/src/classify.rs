//! russh's typed errors as [`Failure`] classes. Nothing here reads text the
//! server sent: a remote program's standard error, a banner, or a disconnect
//! message can never make a failure look like a host-key or auth failure.

use std::io;

use russh::AlgorithmKind;

use crate::{Failure, handler::Offered};

/// The class of a connection that failed with `error`. `offered` is what
/// the host-key check recorded, if it ran; `pinned` is whether the host had
/// pins, which makes "no common host key algorithm" a changed key.
pub(crate) fn classify(error: &russh::Error, offered: Option<&Offered>, pinned: bool) -> Failure {
    use russh::Error as E;
    match error {
        // The check refused the key: only an unpinned host is "unknown".
        E::UnknownKey => match offered {
            Some(Offered::New(_)) => Failure::HostKeyUnknown,
            _ => Failure::HostKeyChanged,
        },
        E::KeyChanged { .. } | E::WrongServerSig => Failure::HostKeyChanged,
        E::NoCommonAlgo {
            kind: AlgorithmKind::Key,
            ..
        } if pinned => Failure::HostKeyChanged,
        E::ConnectionTimeout | E::KeepaliveTimeout | E::InactivityTimeout | E::Elapsed(_) => {
            Failure::Timeout
        }
        E::IO(error) if error.kind() == io::ErrorKind::TimedOut => Failure::Timeout,
        E::IO(_) | E::HUP | E::Disconnect | E::SendError | E::RecvError => Failure::Unreachable,
        E::NotAuthenticated | E::NoAuthMethod | E::UnsupportedAuthMethod => Failure::Auth,
        _ => Failure::Protocol,
    }
}

#[cfg(test)]
mod tests;
