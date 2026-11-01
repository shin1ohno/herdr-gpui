//! Pinned host keys in an OpenSSH `known_hosts` file the app owns. Lookups
//! gather every key of every type for a name, so a host with any pin treats
//! every other key as changed. Writes happen inside a process-wide lock, re-check
//! that the name gained no pin, and replace the file atomically.
//!
//! The file is read and written here rather than through russh's helpers,
//! which report a pinned key of another type as unknown. Only the app writes
//! it, one plain host name per line, so the reader matches plain names
//! (case-insensitively, honouring `!name` negation) and skips what the app
//! never writes: hashed `|1|` names, wildcard patterns, and `@cert-authority`
//! or `@revoked` lines (host certificates are refused anyway).

use std::{
    ffi::OsString,
    fs::{self, DirBuilder, File, OpenOptions},
    io::{self, Read as _, Write as _},
    os::unix::fs::{DirBuilderExt as _, OpenOptionsExt as _},
    path::{Path, PathBuf},
    sync::{
        Mutex, PoisonError,
        atomic::{AtomicU64, Ordering},
    },
};

use crate::{Error, HostKey, Result, Target};

/// Serializes every write in the process, so a check and the write that
/// depends on it cannot interleave with another scene's.
static WRITE_LOCK: Mutex<()> = Mutex::new(());
/// Distinguishes temporary files written by one process.
static NEXT_TEMPORARY: AtomicU64 = AtomicU64::new(0);
/// The largest file read; a pin file is a few hundred bytes per device.
const FILE_LIMIT: usize = 4 * 1024 * 1024;

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
    pub fn pins(&self, target: &Target) -> Result<Pins> {
        let text = self.read()?.unwrap_or_default();
        pins_in(&text, &target.known_hosts_name())
    }

    /// Pins exactly `keys`, which the user approved after a scan found no
    /// pin. Fails with [`crate::Error::PinnedMeanwhile`] if the name gained a
    /// pin since.
    pub fn trust(&self, target: &Target, keys: &[HostKey]) -> Result<()> {
        if keys.is_empty() {
            return Err(Error::Io(io::Error::new(
                io::ErrorKind::InvalidInput,
                "no host key to trust",
            )));
        }
        let name = target.known_hosts_name();
        let _guard = WRITE_LOCK.lock().unwrap_or_else(PoisonError::into_inner);
        let mut text = self.read()?.unwrap_or_default();
        if let Pins::Keys(_) = pins_in(&text, &name)? {
            return Err(Error::PinnedMeanwhile);
        }
        if !text.is_empty() && !text.ends_with('\n') {
            text.push('\n');
        }
        let mut written: Vec<&HostKey> = Vec::with_capacity(keys.len());
        for key in keys {
            if written.contains(&key) {
                continue;
            }
            written.push(key);
            text.push_str(&name);
            text.push(' ');
            text.push_str(&key.known_hosts_fields());
            text.push('\n');
        }
        if text.len() > FILE_LIMIT {
            return Err(known_hosts_error(
                "is too large",
                io::Error::new(io::ErrorKind::InvalidData, "known hosts file over 4 MiB"),
            ));
        }
        self.replace(&text)?;
        tracing::info!(
            category = "host_key",
            keys = written.len(),
            "pinned host keys"
        );
        Ok(())
    }

    /// Removes every pin for `target`, after the user compared the pinned and
    /// offered fingerprints and confirmed.
    pub fn forget(&self, target: &Target) -> Result<()> {
        self.forget_inner(target, None)
    }

    /// Removes only the pins the user reviewed. Another scene changing them
    /// before confirmation requires a fresh review rather than deleting unseen keys.
    pub fn forget_if_matches(&self, target: &Target, expected: &Pins) -> Result<()> {
        self.forget_inner(target, Some(expected))
    }

    fn forget_inner(&self, target: &Target, expected: Option<&Pins>) -> Result<()> {
        let name = target.known_hosts_name();
        let _guard = WRITE_LOCK.lock().unwrap_or_else(PoisonError::into_inner);
        let text = self.read()?.unwrap_or_default();
        if let Some(expected) = expected
            && &pins_in(&text, &name)? != expected
        {
            return Err(Error::PinnedMeanwhile);
        }
        let kept = without_name(&text, &name);
        if kept != text {
            self.replace(&kept)?;
            tracing::info!(category = "host_key", "removed pinned host keys");
        }
        Ok(())
    }

    /// The file's text, or `None` when it does not exist.
    fn read(&self) -> Result<Option<String>> {
        let file = match File::open(&self.path) {
            Ok(file) => file,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(source) => return Err(known_hosts_error("could not be read", source)),
        };
        let mut text = String::new();
        file.take(FILE_LIMIT as u64 + 1)
            .read_to_string(&mut text)
            .map_err(|source| known_hosts_error("could not be read", source))?;
        if text.len() > FILE_LIMIT {
            return Err(known_hosts_error(
                "is too large",
                io::Error::new(io::ErrorKind::InvalidData, "known hosts file over 4 MiB"),
            ));
        }
        Ok(Some(text))
    }

    /// Replaces the file with `text`: a private temporary file in the same
    /// directory, flushed to disk, renamed over the old one, and the
    /// directory flushed, so a crash leaves either the old or the new file.
    fn replace(&self, text: &str) -> Result<()> {
        let written = |source| known_hosts_error("could not be written", source);
        let file_name = self.path.file_name().ok_or_else(|| {
            written(io::Error::new(
                io::ErrorKind::InvalidInput,
                "known hosts path has no file name",
            ))
        })?;
        let directory = match self.path.parent() {
            Some(parent) if !parent.as_os_str().is_empty() => parent,
            _ => Path::new("."),
        };
        DirBuilder::new()
            .recursive(true)
            .mode(0o700)
            .create(directory)
            .map_err(written)?;
        let mut temporary_name = OsString::from(".");
        temporary_name.push(file_name);
        temporary_name.push(format!(
            ".{}.{}.tmp",
            std::process::id(),
            NEXT_TEMPORARY.fetch_add(1, Ordering::Relaxed)
        ));
        let temporary = directory.join(temporary_name);
        let result = write_synced(&temporary, text.as_bytes())
            .and_then(|()| fs::rename(&temporary, &self.path))
            .and_then(|()| File::open(directory)?.sync_all());
        if result.is_err() {
            // Best effort: the file may never have been created.
            let _ = fs::remove_file(&temporary);
        }
        result.map_err(written)
    }
}

fn write_synced(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(path)?;
    file.write_all(bytes)?;
    file.sync_all()
}

fn known_hosts_error(operation: &'static str, source: io::Error) -> Error {
    Error::KnownHosts { operation, source }
}

/// The host names and optional key fields; incomplete entries must still
/// identify the host so a damaged pin fails closed and can be removed.
fn entry(line: &str) -> Option<(&str, Option<&str>, Option<&str>)> {
    let line = line.trim_start();
    if line.is_empty() || line.starts_with('#') || line.starts_with('@') {
        return None;
    }
    let mut fields = line.split_ascii_whitespace();
    Some((fields.next()?, fields.next(), fields.next()))
}

/// Whether the comma-separated `hosts` field names `name`. A `!name`
/// pattern excludes the line, as in OpenSSH.
fn names(hosts: &str, name: &str) -> bool {
    let mut listed = false;
    for pattern in hosts.split(',') {
        if let Some(negated) = pattern.strip_prefix('!') {
            if negated.eq_ignore_ascii_case(name) {
                return false;
            }
        } else if pattern.eq_ignore_ascii_case(name) {
            listed = true;
        }
    }
    listed
}

/// Every key `text` pins under `name`. A line that names the host but whose
/// key cannot be read is an error rather than "no pin", so a damaged pin
/// never turns a pinned host into an unknown one; `forget` still removes it.
fn pins_in(text: &str, name: &str) -> Result<Pins> {
    let mut keys: Vec<HostKey> = Vec::new();
    for (hosts, algorithm, base64) in text.lines().filter_map(entry) {
        if !names(hosts, name) {
            continue;
        }
        let key = algorithm
            .zip(base64)
            .and_then(|(algorithm, base64)| HostKey::from_known_hosts(algorithm, base64));
        let Some(key) = key else {
            return Err(known_hosts_error(
                "has a malformed entry for this host",
                io::Error::new(io::ErrorKind::InvalidData, "malformed host key"),
            ));
        };
        if !keys.contains(&key) {
            keys.push(key);
        }
    }
    Ok(if keys.is_empty() {
        Pins::None
    } else {
        Pins::Keys(keys)
    })
}

/// `text` without `name`: a line naming only it is dropped, and a line that
/// also names other hosts keeps them, with the rest of the line unchanged.
fn without_name(text: &str, name: &str) -> String {
    let mut kept = String::with_capacity(text.len());
    for line in text.split_inclusive('\n') {
        let Some((hosts, _, _)) = entry(line).filter(|(hosts, _, _)| names(hosts, name)) else {
            kept.push_str(line);
            continue;
        };
        let others: Vec<&str> = hosts
            .split(',')
            .filter(|pattern| !pattern.eq_ignore_ascii_case(name))
            .collect();
        if !others.iter().any(|pattern| !pattern.starts_with('!')) {
            continue;
        }
        let start = line.len() - line.trim_start().len();
        let (indent, rest) = line.split_at(start);
        kept.push_str(indent);
        kept.push_str(&others.join(","));
        kept.push_str(rest.get(hosts.len()..).unwrap_or_default());
    }
    kept
}

#[cfg(test)]
mod tests;
