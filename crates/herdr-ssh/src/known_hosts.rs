//! Pinned host keys in an OpenSSH `known_hosts` file the app owns. Lookups
//! gather every key of every type for a name, so a host with any pin treats
//! every other key as changed. Writes happen inside a process-wide lock, re-check
//! that the name gained no pin, and replace the file atomically.

use std::path::{Path, PathBuf};

use crate::{HostKey, Result, Target};

/// What the file pins for one target.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Pins {
    /// Nothing is pinned: the first key needs the user's approval.
    None,
    /// These keys, possibly of several types, are pinned.
    Keys(Vec<HostKey>),
}

/// The app's `known_hosts` file.
#[derive(Debug, Clone)]
pub struct KnownHosts {
    path: PathBuf,
}

impl KnownHosts {
    pub fn new(path: impl AsRef<Path>) -> Self {
        Self {
            path: path.as_ref().to_path_buf(),
        }
    }

    /// Every key pinned under `target`'s known-hosts name. A missing file
    /// pins nothing.
    pub fn pins(&self, _target: &Target) -> Result<Pins> {
        todo!("ssh stream")
    }

    /// Pins exactly `keys`, which the user approved after a scan found no
    /// pin. Fails with [`crate::Error::PinnedMeanwhile`] if the name gained a
    /// pin since.
    pub fn trust(&self, _target: &Target, _keys: &[HostKey]) -> Result<()> {
        todo!("ssh stream")
    }

    /// Removes every pin for `target`, after the user compared the pinned and
    /// offered fingerprints and confirmed.
    pub fn forget(&self, _target: &Target) -> Result<()> {
        todo!("ssh stream")
    }
}
