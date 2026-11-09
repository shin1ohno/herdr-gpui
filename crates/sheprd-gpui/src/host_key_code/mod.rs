//! A computer's SSH host keys, carried out of band to one iPad Add Device attempt.
//!
//! Version 1 is `herdr-ipad://host-key?v=1&fp=<algorithm>:SHA256:<digest>`
//! with one `fp` for each distinct supported SSH key algorithm and an optional
//! `host=<hostname>` hint. Query values use URL form encoding. Digests are the
//! canonical, unpadded standard-base64 SHA-256 of the SSH public-key blob.
//! The hint never selects a connection target. There are no credentials, keys,
//! commands, ports, or connection policy in this format. URL delivery requires
//! an explicit user confirmation before these fingerprints may be used.
//!
//! Inputs are limited to 1 KiB, five algorithms, and a 63-byte ASCII host hint.
//! Unknown parameters/versions, repeated scalar parameters or algorithms,
//! noncanonical digests, fragments, and malformed percent escapes are refused.

#[cfg(not(target_os = "ios"))]
mod dialog;
mod error;
#[cfg(any(target_os = "ios", test))]
mod incoming;
#[cfg(not(target_os = "ios"))]
mod keys;

#[cfg(not(target_os = "ios"))]
pub(crate) use dialog::open;
pub use error::Error;
#[cfg(target_os = "ios")]
pub(crate) use incoming::open_urls;

use base64::{Engine as _, engine::general_purpose::STANDARD_NO_PAD};
use sheprd_client::HostKeyFingerprint;

pub(crate) const MAX_CODE_BYTES: usize = 1024;
const MAX_FINGERPRINTS: usize = 5;
const MAX_HOST_BYTES: usize = 63;
const PREFIX: &str = "herdr-ipad://host-key?";

/// An algorithm is the SSH wire key type, including the exact ECDSA curve.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Fingerprint {
    algorithm: String,
    fingerprint: String,
}

impl Fingerprint {
    pub(crate) fn algorithm(&self) -> &str {
        &self.algorithm
    }

    pub(crate) fn fingerprint(&self) -> &str {
        &self.fingerprint
    }

    fn parse(value: &str) -> Result<Self, Error> {
        let (algorithm, fingerprint) = value.split_once(':').ok_or(Error::Fingerprints)?;
        if !supported_algorithm(algorithm) {
            return Err(Error::Fingerprints);
        }
        let digest = fingerprint
            .strip_prefix("SHA256:")
            .ok_or(Error::Fingerprints)?;
        let bytes = STANDARD_NO_PAD
            .decode(digest)
            .map_err(|_| Error::Fingerprints)?;
        if bytes.len() != 32 || STANDARD_NO_PAD.encode(&bytes) != digest {
            return Err(Error::Fingerprints);
        }
        Ok(Self {
            algorithm: algorithm.to_owned(),
            fingerprint: fingerprint.to_owned(),
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Code {
    fingerprints: Vec<Fingerprint>,
    host_hint: Option<String>,
}

impl Code {
    pub(crate) fn parse(text: &str) -> Result<Self, Error> {
        if text.len() > MAX_CODE_BYTES {
            return Err(Error::TooLarge);
        }
        let query = text.strip_prefix(PREFIX).ok_or(Error::Invalid)?;
        if query.is_empty() || query.contains('#') || !valid_percent_escapes(query.as_bytes()) {
            return Err(Error::Invalid);
        }
        let mut version = false;
        let mut host_hint = None;
        let mut fingerprints = Vec::<Fingerprint>::new();
        for parameter in query.split('&') {
            // Split before decoding: an escaped '&' is always part of a value.
            let (name, _) = parameter.split_once('=').ok_or(Error::Invalid)?;
            let mut pair = url::form_urlencoded::parse(parameter.as_bytes());
            let (_, value) = pair.next().ok_or(Error::Invalid)?;
            match name {
                "v" if !version => {
                    if value != "1" {
                        return Err(Error::Version);
                    }
                    version = true;
                }
                "host" if host_hint.is_none() => {
                    if !valid_host_hint(&value) {
                        return Err(Error::HostHint);
                    }
                    host_hint = Some(value.into_owned());
                }
                "fp" => {
                    let fingerprint = Fingerprint::parse(&value)?;
                    if fingerprints.len() == MAX_FINGERPRINTS
                        || fingerprints
                            .iter()
                            .any(|known| known.algorithm == fingerprint.algorithm)
                    {
                        return Err(Error::Fingerprints);
                    }
                    fingerprints.push(fingerprint);
                }
                _ => return Err(Error::Invalid),
            }
        }
        if !version {
            return Err(Error::Version);
        }
        if fingerprints.is_empty() {
            return Err(Error::Fingerprints);
        }
        Ok(Self {
            fingerprints,
            host_hint,
        })
    }

    #[cfg(any(not(target_os = "ios"), test))]
    pub(crate) fn to_url(&self) -> String {
        let mut query = url::form_urlencoded::Serializer::new(String::new());
        query.append_pair("v", "1");
        for key in &self.fingerprints {
            query.append_pair("fp", &format!("{}:{}", key.algorithm, key.fingerprint));
        }
        if let Some(host) = &self.host_hint {
            query.append_pair("host", host);
        }
        format!("{PREFIX}{}", query.finish())
    }

    pub(crate) fn fingerprints(&self) -> &[Fingerprint] {
        &self.fingerprints
    }

    pub(crate) fn host_hint(&self) -> Option<&str> {
        self.host_hint.as_deref()
    }

    /// `describe` derives these friendly names and bit sizes from the exact
    /// wire algorithm. Requiring the size preserves ECDSA curve identity.
    pub(crate) fn matches(&self, scanned: &[HostKeyFingerprint]) -> bool {
        !scanned.is_empty()
            && scanned.iter().all(|offered| {
                let algorithm = match (offered.kind.as_str(), offered.bits) {
                    ("ED25519", 256) => "ssh-ed25519",
                    ("ECDSA", 256) => "ecdsa-sha2-nistp256",
                    ("ECDSA", 384) => "ecdsa-sha2-nistp384",
                    ("ECDSA", 521) => "ecdsa-sha2-nistp521",
                    ("RSA", _) => "ssh-rsa",
                    _ => return false,
                };
                self.fingerprints.iter().any(|expected| {
                    expected.algorithm == algorithm && expected.fingerprint == offered.fingerprint
                })
            })
    }
}

fn supported_algorithm(algorithm: &str) -> bool {
    matches!(
        algorithm,
        "ssh-ed25519"
            | "ecdsa-sha2-nistp256"
            | "ecdsa-sha2-nistp384"
            | "ecdsa-sha2-nistp521"
            | "ssh-rsa"
    )
}

fn valid_host_hint(host: &str) -> bool {
    !host.is_empty()
        && host.len() <= MAX_HOST_BYTES
        && host
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.'))
        && host
            .bytes()
            .next()
            .is_some_and(|byte| byte.is_ascii_alphanumeric())
}

fn valid_percent_escapes(mut bytes: &[u8]) -> bool {
    while let Some((&first, rest)) = bytes.split_first() {
        if first == b'%' {
            let Some((hex, rest)) = rest.split_first_chunk::<2>() else {
                return false;
            };
            if !hex.iter().all(u8::is_ascii_hexdigit) {
                return false;
            }
            bytes = rest;
        } else {
            if !first.is_ascii_graphic() {
                return false;
            }
            bytes = rest;
        }
    }
    true
}

#[cfg(test)]
mod tests;
