//! The device key in the real Keychain. On the simulator the key is a
//! software key with the device's access class, so these check the lookup,
//! creation, attributes, and signatures a device shares with it; only a
//! device can show that the key is in the Secure Enclave.
//!
//! The Keychain refuses a process without an access group
//! (`errSecMissingEntitlement`), and the simulator reads a test binary's
//! from its `__TEXT,__entitlements` section, so these tests and
//! `ios_keychain`'s are ignored unless the binary is linked with one:
//!
//! ```sh
//! cargo rustc -p sheprd-credentials --tests --profile test --target aarch64-apple-ios-sim -- \
//!   -C link-arg=-Wl,-sectcreate,__TEXT,__entitlements,crates/sheprd-credentials/simulator.entitlements
//! xcrun simctl spawn booted target/aarch64-apple-ios-sim/debug/deps/sheprd_credentials-HASH --include-ignored
//! ```
use super::*;
use crate::{
    device_key::keychain::tagged,
    sec_item::tests::{device_only, stored, synchronizable},
};
use p256::ecdsa::{Signature, VerifyingKey, signature::Verifier};

#[test]
#[ignore = "needs a Keychain access group; see this module's docs"]
fn the_key_is_created_once_kept_on_this_device_and_signs_verifiably() {
    let label = Label::new();
    let key = DeviceKey::load_or_create(&label.0).unwrap();
    assert_eq!(
        DeviceKey::load_or_create(&label.0).unwrap().public_sec1(),
        key.public_sec1()
    );
    let items = stored(tagged(&label.0));
    assert_eq!(items.len(), 1);
    assert!(device_only(&items[0]));
    assert_eq!(synchronizable(&items[0]), Some(false));

    let public = VerifyingKey::from_sec1_bytes(&key.public_sec1()).unwrap();
    for message in [b"".as_slice(), b"session id and userauth request"] {
        let signature = Signature::from_slice(&key.sign(message).unwrap()).unwrap();
        public.verify(message, &signature).unwrap();
    }

    DeviceKey::delete(&label.0).unwrap();
    assert!(stored(tagged(&label.0)).is_empty());
    // Deleting nothing succeeds.
    DeviceKey::delete(&label.0).unwrap();
    let replaced = DeviceKey::load_or_create(&label.0).unwrap();
    assert_ne!(replaced.public_sec1(), key.public_sec1());
}

/// Scenes that ask for the key at the same moment all get the same one, and
/// the Keychain holds exactly one. Without the lock in `load_or_create` this
/// stored five to eight keys under the label in the simulator.
#[test]
#[ignore = "needs a Keychain access group; see this module's docs"]
fn concurrent_creators_end_with_one_key() {
    let label = Label::new();
    let keys: Vec<[u8; 65]> = std::thread::scope(|scope| {
        let creators: Vec<_> = (0..8)
            .map(|_| scope.spawn(|| DeviceKey::load_or_create(&label.0).unwrap().public_sec1()))
            .collect();
        creators
            .into_iter()
            .map(|creator| creator.join().unwrap())
            .collect()
    });
    assert!(keys.windows(2).all(|pair| pair[0] == pair[1]));
    assert_eq!(stored(tagged(&label.0)).len(), 1);
}
