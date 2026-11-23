//! iPad's in-process SSH, `herdr_ssh::Failure`, `herdr_ssh::Error`, and
//! `herdr_ssh::forward::Error`: `error-ssh.ftl`.

use super::Localize;
use crate::i18n::fl;
use herdr_ssh::{Error, Failure, forward};

impl Localize for Failure {
    fn localize(&self) -> String {
        match self {
            // keep-sorted start
            Self::Auth => fl!("error-ssh-auth"),
            Self::Cancelled => fl!("error-ssh-cancelled"),
            Self::HostKeyChanged => fl!("error-ssh-host-key-changed"),
            Self::HostKeyUnknown => fl!("error-ssh-host-key-unknown"),
            Self::Protocol => fl!("error-ssh-protocol"),
            Self::Timeout => fl!("error-ssh-timeout"),
            Self::Unreachable => fl!("error-ssh-unreachable"),
            // keep-sorted end
        }
    }
}

impl Localize for Error {
    fn localize(&self) -> String {
        match self {
            // keep-sorted start
            Self::Failure(failure) => failure.localize(),
            Self::Forward(error) => error.localize(),
            // The reasons and operations are `herdr-ssh`'s own words, kept as
            // they are.
            Self::InvalidClientKey { reason } => {
                fl!("error-ssh-invalid-client-key", reason = *reason)
            }
            Self::InvalidTarget { reason } => fl!("error-ssh-invalid-target", reason = *reason),
            // The system's own words.
            Self::Io(error) => error.to_string(),
            Self::KnownHosts { operation, .. } => {
                fl!("error-ssh-known-hosts", operation = *operation)
            }
            Self::PinnedMeanwhile => fl!("error-ssh-pinned-meanwhile"),
            // keep-sorted end
        }
    }
}

impl Localize for forward::Error {
    fn localize(&self) -> String {
        match self {
            // keep-sorted start
            Self::Busy => fl!("error-ssh-forward-busy"),
            Self::Connect(failure) => {
                fl!("error-ssh-forward-connect", failure = failure.localize())
            }
            Self::ConnectionLost(failure) => {
                fl!(
                    "error-ssh-forward-connection-lost",
                    failure = failure.localize()
                )
            }
            Self::InvalidPort => fl!("error-ssh-forward-invalid-port"),
            // The system's own words.
            Self::Io(error) => error.to_string(),
            Self::NotLoopback => fl!("error-ssh-forward-not-loopback"),
            Self::PortClosed => fl!("error-ssh-forward-port-closed"),
            Self::Prohibited => fl!("error-ssh-forward-prohibited"),
            Self::Refused => fl!("error-ssh-forward-refused"),
            Self::Stopped => fl!("error-ssh-forward-stopped"),
            Self::Timeout => fl!("error-ssh-forward-timeout"),
            // keep-sorted end
        }
    }
}
