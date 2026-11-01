//! russh's client configuration: the algorithm allow-list (docs/ios.md,
//! "Algorithms") and the desktop `ssh` policy's timeouts.

use std::{borrow::Cow, sync::Arc, time::Duration};

use russh::{
    Preferred, cipher,
    client::Config,
    compression, kex,
    keys::{Algorithm, EcdsaCurve, HashAlg},
    mac,
};

use crate::HostKey;

/// TCP connect and key exchange (`ConnectTimeout=10`).
pub(crate) const CONNECT_DEADLINE: Duration = Duration::from_secs(10);
/// Authentication and opening the channel, after key exchange.
pub(crate) const SETUP_DEADLINE: Duration = Duration::from_secs(20);
/// `ServerAliveInterval=15` and `ServerAliveCountMax=4`: russh, like
/// OpenSSH, gives up when the count of unanswered keepalives exceeds the
/// maximum.
const KEEPALIVE_INTERVAL: Duration = Duration::from_secs(15);
const KEEPALIVE_MAX: usize = 4;

/// `ext-info-c` and `kex-strict-c-v00@openssh.com` are required: without
/// strict key exchange chacha20-poly1305 is open to Terrapin.
const KEX: &[kex::Name] = &[
    kex::MLKEM768X25519_SHA256,
    kex::CURVE25519,
    kex::CURVE25519_PRE_RFC_8731,
    kex::EXTENSION_SUPPORT_AS_CLIENT,
    kex::EXTENSION_OPENSSH_STRICT_KEX_AS_CLIENT,
];
/// Host key algorithms, most preferred first. RSA signatures use SHA-2 only;
/// the key itself is encoded as `ssh-rsa` even when its signature uses SHA-2.
const HOST_KEYS: &[Algorithm] = &[
    Algorithm::Ed25519,
    Algorithm::Ecdsa {
        curve: EcdsaCurve::NistP256,
    },
    Algorithm::Ecdsa {
        curve: EcdsaCurve::NistP384,
    },
    Algorithm::Ecdsa {
        curve: EcdsaCurve::NistP521,
    },
    Algorithm::Rsa {
        hash: Some(HashAlg::Sha512),
    },
    Algorithm::Rsa {
        hash: Some(HashAlg::Sha256),
    },
];
const CIPHERS: &[cipher::Name] = &[cipher::CHACHA20_POLY1305, cipher::AES_256_GCM];
/// Only consulted for a cipher without its own authentication; both ciphers
/// above are AEADs.
const MACS: &[mac::Name] = &[mac::HMAC_SHA512_ETM, mac::HMAC_SHA256_ETM];
const COMPRESSION: &[compression::Name] = &[compression::NONE];

/// The allow-listed host key algorithms that verify a pinned key, in
/// allow-list order: a connection to a pinned host negotiates only these.
/// Empty when no pin has an allow-listed type.
pub(crate) fn pinned_host_keys(pins: &[HostKey]) -> Vec<Algorithm> {
    HOST_KEYS
        .iter()
        .filter(|algorithm| {
            let key_type = match algorithm {
                Algorithm::Rsa { .. } => "ssh-rsa",
                algorithm => algorithm.as_str(),
            };
            pins.iter().any(|pin| pin.algorithm() == key_type)
        })
        .cloned()
        .collect()
}

/// For a scan: the pinned types first, so a host that still has its pinned
/// key shows it, then every other allow-listed type, so a host that dropped
/// it shows what it offers instead.
pub(crate) fn scan_host_keys(pins: &[HostKey]) -> Vec<Algorithm> {
    let mut algorithms = pinned_host_keys(pins);
    for algorithm in HOST_KEYS {
        if !algorithms.contains(algorithm) {
            algorithms.push(algorithm.clone());
        }
    }
    algorithms
}

/// The client configuration offering `host_keys`. No certificate types are
/// offered, so a host can only present a plain key.
pub(crate) fn config(host_keys: Vec<Algorithm>) -> Arc<Config> {
    Arc::new(Config {
        preferred: Preferred {
            kex: Cow::Borrowed(KEX),
            key: Cow::Owned(host_keys),
            host_key_certificates: Cow::Borrowed(&[]),
            cipher: Cow::Borrowed(CIPHERS),
            mac: Cow::Borrowed(MACS),
            compression: Cow::Borrowed(COMPRESSION),
        },
        inactivity_timeout: None,
        keepalive_interval: Some(KEEPALIVE_INTERVAL),
        keepalive_max: KEEPALIVE_MAX,
        nodelay: true,
        ..Config::default()
    })
}

#[cfg(test)]
mod tests;
