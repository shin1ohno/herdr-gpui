#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;

mod payload;

fn fingerprint(algorithm: &str, digest: u8) -> Fingerprint {
    Fingerprint {
        algorithm: algorithm.into(),
        fingerprint: format!("SHA256:{}", STANDARD_NO_PAD.encode([digest; 32])),
    }
}

pub(super) fn code() -> Code {
    Code {
        fingerprints: vec![
            fingerprint("ssh-ed25519", 1),
            fingerprint("ecdsa-sha2-nistp256", 2),
        ],
        host_hint: Some("computer.local".into()),
    }
}
