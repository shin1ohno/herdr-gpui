//! Why the iPad's in-process SSH client could not start. Connection failures
//! are not here: they are `SshFailure`s, as on the desktop.

use std::io;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    // keep-sorted start
    /// This installation's SSH key could not be loaded or created, such as
    /// while the device is locked.
    #[error(transparent)]
    DeviceKey(#[from] sheprd_credentials::Error),
    /// Nothing can start `ssh` or an application-built command on iPadOS.
    #[error("this device cannot start programs to reach a host")]
    ProcessUnsupported,
    /// A local failure of the client: an invalid target or the known hosts
    /// file. Its text never carries what the server sent.
    #[error(transparent)]
    Ssh(#[from] herdr_ssh::Error),
    #[error("no state directory to keep known hosts in")]
    StateDir,
    // keep-sorted end
}

impl Error {
    /// The transport retry/cancellation category `crate::Error::kind` reports.
    pub(crate) fn kind(&self) -> io::ErrorKind {
        match self {
            Self::DeviceKey(_) => io::ErrorKind::PermissionDenied,
            Self::ProcessUnsupported => io::ErrorKind::Unsupported,
            Self::Ssh(
                herdr_ssh::Error::InvalidTarget { .. } | herdr_ssh::Error::InvalidClientKey { .. },
            ) => io::ErrorKind::InvalidInput,
            Self::Ssh(
                herdr_ssh::Error::KnownHosts { source, .. } | herdr_ssh::Error::Io(source),
            ) => source.kind(),
            Self::Ssh(herdr_ssh::Error::PinnedMeanwhile) => io::ErrorKind::PermissionDenied,
            Self::Ssh(herdr_ssh::Error::Failure(_)) => io::ErrorKind::NotConnected,
            Self::StateDir => io::ErrorKind::NotFound,
        }
    }
}

impl From<Error> for crate::Error {
    fn from(error: Error) -> Self {
        Self::Ssh(error.into())
    }
}
