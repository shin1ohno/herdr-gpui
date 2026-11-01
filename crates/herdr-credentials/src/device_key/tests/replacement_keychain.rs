//! Requires the same simulator Keychain entitlement as `keychain` tests.
use super::*;
use crate::{
    device_key::{
        keychain::tagged,
        replace::{Slot, Tags},
    },
    sec_item::tests::{device_only, stored, synchronizable},
};
use p256::ecdsa::{Signature, VerifyingKey, signature::Verifier};

#[test]
#[ignore = "needs a Keychain access group; see keychain module's docs"]
fn replacement_is_stored_under_the_active_tag_and_keeps_device_only_protection() {
    let label = Label::new();
    let old = DeviceKey::load_or_create(&label.0).unwrap();
    let new = DeviceKey::replace(&label.0).unwrap();
    assert_ne!(new.public_sec1(), old.public_sec1());
    assert_eq!(
        DeviceKey::load_or_create(&label.0).unwrap().public_sec1(),
        new.public_sec1()
    );
    let items = stored(tagged(&label.0));
    assert_eq!(items.len(), 1);
    assert!(device_only(&items[0]));
    assert_eq!(synchronizable(&items[0]), Some(false));
    let tags = Tags::new(&label.0);
    for slot in [Slot::Staged, Slot::Retired] {
        assert!(stored(tagged(tags.tag(slot))).is_empty());
    }
    let public = VerifyingKey::from_sec1_bytes(&new.public_sec1()).unwrap();
    let message = b"authentication after key replacement";
    let signature = Signature::from_slice(&new.sign(message).unwrap()).unwrap();
    public.verify(message, &signature).unwrap();
}
