//! Failures while reading, displaying, or accepting an SSH host-key code.

use std::{io, path::PathBuf};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    // keep-sorted start
    #[error(
        "The host-key code must contain distinct supported SSH key types and SHA-256 fingerprints"
    )]
    Fingerprints,
    #[error("The host name hint in the host-key code is invalid")]
    HostHint,
    #[error("The host-key code is not a valid herdr-ipad host-key URL")]
    Invalid,
    #[error(
        "No SSH host public keys were found; enable Remote Login or install and configure OpenSSH on this computer"
    )]
    NoHostKeys,
    #[error("The SSH host public key at {path} is invalid")]
    PublicKey { path: PathBuf },
    #[error(transparent)]
    Qr(#[from] herdr_remote_control::app::Error),
    #[error("Could not read the SSH host public key at {path}: {source}")]
    Read {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("The host-key code is too large")]
    TooLarge,
    #[error("SSH host public keys are unavailable on this platform")]
    Unsupported,
    #[error("The host-key code version is not supported")]
    Version,
    // keep-sorted end
}
