//! A host's public key as the wire carries it, compared byte for byte.

/// One host public key: its algorithm name (`ssh-ed25519`,
/// `ecdsa-sha2-nistp256`, `rsa-sha2-512` keys are stored as `ssh-rsa`) and
/// the SSH wire encoding of the key. Equality is exact byte equality.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct HostKey {
    algorithm: String,
    blob: Vec<u8>,
}

impl HostKey {
    pub fn algorithm(&self) -> &str {
        &self.algorithm
    }

    /// `SHA256:<unpadded base64>`, as `ssh-keygen -l` prints it.
    pub fn fingerprint(&self) -> String {
        todo!("ssh stream")
    }
}
