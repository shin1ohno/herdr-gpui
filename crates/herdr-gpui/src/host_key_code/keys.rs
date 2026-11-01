//! Read only the fixed public host-key files, away from the UI thread.

use super::{Code, Error, Fingerprint, valid_host_hint};
use base64::{
    Engine as _,
    engine::general_purpose::{STANDARD, STANDARD_NO_PAD},
};
use sha2::{Digest as _, Sha256};
use std::{
    fs::File,
    io::{self, Read},
    path::{Path, PathBuf},
};

const PUBLIC_KEY_BYTES: u64 = 16 * 1024;
const HOST_FILES: &[&str] = &[
    "ssh_host_ed25519_key.pub",
    "ssh_host_ecdsa_key.pub",
    "ssh_host_rsa_key.pub",
];

pub(super) fn read_computer() -> Result<Code, Error> {
    #[cfg(unix)]
    let directory = PathBuf::from("/etc/ssh");
    #[cfg(windows)]
    let directory = std::env::var_os("ProgramData")
        .map(|root| PathBuf::from(root).join("ssh"))
        .filter(|path| path.is_absolute())
        .ok_or(Error::Unsupported)?;
    #[cfg(not(any(unix, windows)))]
    return Err(Error::Unsupported);
    #[cfg(any(unix, windows))]
    {
        let hint = crate::remote::nearby::host_name();
        read_in(&directory, valid_host_hint(&hint).then_some(hint))
    }
}

fn read_in(directory: &Path, host_hint: Option<String>) -> Result<Code, Error> {
    let mut fingerprints = Vec::new();
    for name in HOST_FILES {
        let path = directory.join(name);
        let file = match File::open(&path) {
            Ok(file) => file,
            Err(error) if error.kind() == io::ErrorKind::NotFound => continue,
            Err(source) => return Err(Error::Read { path, source }),
        };
        let mut bytes = Vec::new();
        file.take(PUBLIC_KEY_BYTES + 1)
            .read_to_end(&mut bytes)
            .map_err(|source| Error::Read {
                path: path.clone(),
                source,
            })?;
        if bytes.len() as u64 > PUBLIC_KEY_BYTES {
            return Err(Error::PublicKey { path });
        }
        let key = public_key(&bytes).ok_or_else(|| Error::PublicKey { path })?;
        // sshd has one of these files per family. Refuse a misconfigured
        // duplicate rather than silently discarding any advertised identity.
        if fingerprints
            .iter()
            .any(|known: &Fingerprint| known.algorithm == key.algorithm)
        {
            return Err(Error::Fingerprints);
        }
        fingerprints.push(key);
    }
    if fingerprints.is_empty() {
        return Err(Error::NoHostKeys);
    }
    Ok(Code {
        fingerprints,
        host_hint,
    })
}

fn public_key(bytes: &[u8]) -> Option<Fingerprint> {
    let text = std::str::from_utf8(bytes).ok()?;
    let mut lines = text.lines().filter(|line| !line.trim().is_empty());
    let line = lines.next()?;
    if lines.next().is_some() || line.chars().any(|c| c.is_control() && c != '\t') {
        return None;
    }
    let mut words = line.split_whitespace();
    let algorithm = words.next()?;
    if !super::supported_algorithm(algorithm) {
        return None;
    }
    let blob = STANDARD.decode(words.next()?).ok()?;
    // The textual type and the type hashed inside the SSH blob must agree.
    let (wire_type, body) = wire_string(&blob)?;
    if wire_type != algorithm.as_bytes() || !valid_body(algorithm, body) {
        return None;
    }
    Some(Fingerprint {
        algorithm: algorithm.to_owned(),
        fingerprint: format!("SHA256:{}", STANDARD_NO_PAD.encode(Sha256::digest(&blob))),
    })
}

fn wire_string(bytes: &[u8]) -> Option<(&[u8], &[u8])> {
    let (length, rest) = bytes.split_first_chunk::<4>()?;
    let length = usize::try_from(u32::from_be_bytes(*length)).ok()?;
    (length <= rest.len()).then(|| rest.split_at(length))
}

fn valid_body(algorithm: &str, bytes: &[u8]) -> bool {
    let Some((first, rest)) = wire_string(bytes) else {
        return false;
    };
    if algorithm == "ssh-ed25519" {
        return first.len() == 32 && rest.is_empty();
    }
    let Some((second, rest)) = wire_string(rest) else {
        return false;
    };
    if !rest.is_empty() {
        return false;
    }
    match algorithm {
        "ssh-rsa" => !first.is_empty() && !second.is_empty(),
        "ecdsa-sha2-nistp256" => {
            first == b"nistp256" && second.len() == 65 && second.first() == Some(&4)
        }
        "ecdsa-sha2-nistp384" => {
            first == b"nistp384" && second.len() == 97 && second.first() == Some(&4)
        }
        "ecdsa-sha2-nistp521" => {
            first == b"nistp521" && second.len() == 133 && second.first() == Some(&4)
        }
        _ => false,
    }
}

#[cfg(test)]
#[path = "tests/keys.rs"]
mod tests;
