//! Host keys on iPadOS, pinned in the app's own known hosts file through the
//! in-process client (docs/ios.md, "Host keys"). A host with any pin treats
//! every other key, of any type, as changed and refused; only a host with
//! none is offered for approval, and only from Add Device. Approval pins the
//! exact keys the scan saw, inside the client's lock, after checking the host
//! gained no pin meanwhile. A changed key is resolved only by forgetting the
//! pins after the user compared them with what the host offers now.
use super::{HostKeyFingerprint, HostKeyScan, NewHostKey};
use crate::ssh::{Error, SshFailure, in_process, ipad};
use crate::{Result, catalog::validate_target};
use herdr_ssh::{HostKey, Pins, Scan, Target};
use std::sync::atomic::AtomicBool;

/// Connects to `target` without authenticating, only to learn its host key
/// against the pins, so an unverified host never learns the user name.
/// Blocks on the network: call it from a background thread.
pub fn scan_host_key(target: &str) -> Result<HostKeyScan> {
    validate_target(target)?;
    let parsed = in_process::parse(target)?;
    let known_hosts = in_process::known_hosts_path()?;
    let pins = herdr_ssh::KnownHosts::new(&known_hosts)
        .pins(&parsed)
        .map_err(in_process::Error::Ssh)?;
    Ok(match scan(&parsed, &pins)? {
        Ok(Scan::Trusted) => HostKeyScan::Trusted,
        Ok(Scan::New(keys)) => {
            tracing::info!(
                category = "host_key",
                keys = keys.len(),
                "host offered a new key"
            );
            HostKeyScan::New(new_key(parsed, keys, known_hosts)?)
        }
        Ok(Scan::Changed { .. }) => {
            tracing::warn!(category = "host_key", "host offered a changed key");
            HostKeyScan::Refused(SshFailure::HostKey)
        }
        Err(failure) => {
            tracing::warn!(category = "host_key", ?failure, "host key check failed");
            HostKeyScan::Refused(failure)
        }
    })
}

/// Pins the keys the user approved. A host that gained a pin since the scan
/// is refused as a changed key, never overwritten. Blocks on disk: call it
/// from a background thread.
pub fn trust_host_key(key: &NewHostKey) -> Result<()> {
    match herdr_ssh::KnownHosts::new(&key.known_hosts).trust(&key.target, &key.keys) {
        Ok(()) => {
            tracing::info!(category = "host_key", "trusted a new host key");
            Ok(())
        }
        Err(herdr_ssh::Error::PinnedMeanwhile) => {
            tracing::warn!(
                category = "host_key",
                "host gained a trusted key before approval"
            );
            Err(Error::Refused(SshFailure::HostKey).into())
        }
        Err(error) => Err(in_process::Error::Ssh(error).into()),
    }
}

/// A host whose offered key matches none of its pins, for the user to
/// compare before forgetting the pins.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostKeyChange {
    /// Every key pinned for the host now.
    pub pinned: Vec<HostKeyFingerprint>,
    /// What the host offers instead. Trusting it with [`trust_host_key`]
    /// succeeds only once the pins are forgotten.
    pub offered: NewHostKey,
    /// The exact pins the user reviewed, rechecked before forgetting them.
    pins: Pins,
}

/// What `target` offers against what is pinned for it, or `None` when its
/// key is not a changed one: it matches a pin, or nothing is pinned. Blocks
/// on the network: call it from a background thread.
pub fn changed_host_key(target: &str) -> Result<Option<HostKeyChange>> {
    validate_target(target)?;
    let parsed = in_process::parse(target)?;
    let known_hosts = in_process::known_hosts_path()?;
    let pins = herdr_ssh::KnownHosts::new(&known_hosts)
        .pins(&parsed)
        .map_err(in_process::Error::Ssh)?;
    let Pins::Keys(pinned) = &pins else {
        return Ok(None);
    };
    let pinned = pinned.iter().map(describe).collect();
    match scan(&parsed, &pins)? {
        Ok(Scan::Changed { offered }) => Ok(Some(HostKeyChange {
            pinned,
            offered: new_key(parsed, vec![offered], known_hosts)?,
            pins,
        })),
        Ok(Scan::Trusted | Scan::New(_)) => Ok(None),
        Err(failure) => Err(Error::Refused(failure).into()),
    }
}

/// Removes the exact keys the user compared with
/// the [`HostKeyChange`] and confirmed. The next connection then fails as an
/// untrusted host until a key is approved. Blocks on disk: call it from a
/// background thread. A concurrent change refuses the stale confirmation.
pub fn forget_host_key(change: &HostKeyChange) -> Result<()> {
    herdr_ssh::KnownHosts::new(&change.offered.known_hosts)
        .forget_if_matches(&change.offered.target, &change.pins)
        .map_err(in_process::Error::Ssh)?;
    tracing::info!(category = "host_key", "forgot a host's pinned keys");
    Ok(())
}

/// The scan, with a connection failure as its class. Nothing stops it but
/// its own deadline, as the desktop scan's.
fn scan(target: &Target, pins: &Pins) -> Result<std::result::Result<Scan, SshFailure>> {
    match herdr_ssh::scan(target, pins, &AtomicBool::new(false)) {
        Ok(scan) => Ok(Ok(scan)),
        Err(herdr_ssh::Error::Failure(failure)) => Ok(Err(ipad::ssh_failure(failure))),
        Err(error) => Err(in_process::Error::Ssh(error).into()),
    }
}

fn new_key(
    target: Target,
    keys: Vec<HostKey>,
    known_hosts: std::path::PathBuf,
) -> Result<NewHostKey> {
    if keys.is_empty() {
        return Err(crate::Error::RemoteOutput);
    }
    let mut fingerprints: Vec<HostKeyFingerprint> = Vec::new();
    for fingerprint in keys.iter().map(describe) {
        if !fingerprints
            .iter()
            .any(|known| known.fingerprint == fingerprint.fingerprint)
        {
            fingerprints.push(fingerprint);
        }
    }
    Ok(NewHostKey {
        fingerprints,
        known_hosts,
        target,
        keys,
    })
}

/// One key as `ssh-keygen -l` describes it. The in-process client reports
/// no size for an RSA key, so its `bits` is 0.
fn describe(key: &HostKey) -> HostKeyFingerprint {
    let (kind, bits) = match ipad::key_kind(key.algorithm()) {
        Some(kind) => kind,
        None => (key.algorithm(), 0),
    };
    HostKeyFingerprint {
        kind: kind.to_owned(),
        bits,
        fingerprint: key.fingerprint(),
    }
}
