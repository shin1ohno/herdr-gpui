//! iPad's in-process SSH, `herdr_ssh::Failure` and `herdr_ssh::Error`:
//! `error-ssh.ftl`.

use super::Localize;
use crate::i18n::fl;
use herdr_ssh::{Error, Failure};

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
