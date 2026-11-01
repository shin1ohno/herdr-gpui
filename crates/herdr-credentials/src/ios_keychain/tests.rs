//! Saved sign-ins in the real Keychain. Ignored unless the test binary has a
//! Keychain access group; see `device_key`'s Keychain tests for how to run
//! them in the simulator.
#![allow(clippy::unwrap_used)]
use super::*;
use crate::sec_item::tests::{device_only, stored, synchronizable};
use std::sync::atomic::{AtomicUsize, Ordering};

const SERVICE: &str = "dev.herdr.gpui.test.sign-in";

/// An account no other test or run uses, whose entry is removed when the
/// test ends, passed or failed.
struct Account(String);

impl Account {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        Self(format!(
            "{}.{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ))
    }

    fn saved(&self) -> Option<String> {
        read(SERVICE, &self.0)
            .unwrap()
            .map(|token| token.expose_secret().to_owned())
    }
}

impl Drop for Account {
    fn drop(&mut self) {
        let _ = save(SERVICE, &self.0, None);
    }
}

#[test]
#[ignore = "needs a Keychain access group; see device_key's Keychain tests"]
fn a_sign_in_is_kept_on_this_device_and_replaced_in_place() {
    let account = Account::new();
    assert_eq!(account.saved(), None);
    save(SERVICE, &account.0, Some(&"first".into())).unwrap();
    assert_eq!(account.saved().as_deref(), Some("first"));
    save(SERVICE, &account.0, Some(&"second".into())).unwrap();
    assert_eq!(account.saved().as_deref(), Some("second"));

    let items = stored(entry(SERVICE, &account.0));
    assert_eq!(items.len(), 1);
    assert!(device_only(&items[0]));
    assert_eq!(synchronizable(&items[0]), Some(false));

    save(SERVICE, &account.0, None).unwrap();
    assert_eq!(account.saved(), None);
    // Removing nothing succeeds.
    save(SERVICE, &account.0, None).unwrap();
}

/// An entry kept with any other class, here the default one, is never given
/// a new secret: the save fails and leaves the entry as it was.
#[test]
#[ignore = "needs a Keychain access group; see device_key's Keychain tests"]
fn an_entry_with_another_class_is_not_given_a_new_secret() {
    let account = Account::new();
    let other = entry(SERVICE, &account.0).set(Name::ValueData, CFData::from_buffer(b"old"));
    sec_item::add(&other).unwrap();
    assert!(!device_only(&stored(entry(SERVICE, &account.0))[0]));
    assert!(matches!(
        save(SERVICE, &account.0, Some(&"new".into())),
        Err(crate::Error::IosKeychain(Error::Write(_)))
    ));
    assert_eq!(account.saved().as_deref(), Some("old"));
}
