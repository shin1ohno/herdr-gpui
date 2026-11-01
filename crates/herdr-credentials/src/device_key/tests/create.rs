use super::*;
use crate::device_key::create::find_or_create;

type Lookup = Result<Option<&'static str>, Error>;

/// Runs `find_or_create` against a Keychain stand-in whose lookups return
/// `lookups` in order, and counts the lookups and creations it made.
fn run(
    lookups: Vec<Lookup>,
    create: Result<(), Error>,
) -> (Result<&'static str, Error>, usize, usize) {
    let mut lookups = lookups.into_iter();
    let (mut looked, mut created) = (0, 0);
    let result = find_or_create(
        || {
            looked += 1;
            lookups.next().unwrap()
        },
        || {
            created += 1;
            create
        },
    );
    (result, looked, created)
}

#[test]
fn an_existing_key_is_used_without_creating_one() {
    let (key, looked, created) = run(vec![Ok(Some("stored"))], Ok(()));
    assert_eq!(key.unwrap(), "stored");
    assert_eq!((looked, created), (1, 0));
}

#[test]
fn a_locked_device_is_a_transient_error_and_never_creates_a_key() {
    let (key, looked, created) = run(vec![Err(Error::Locked)], Ok(()));
    assert!(matches!(key, Err(Error::Locked)));
    assert_eq!((looked, created), (1, 0));
}

#[test]
fn a_new_key_is_read_back_from_the_keychain() {
    let (key, looked, created) = run(vec![Ok(None), Ok(Some("stored"))], Ok(()));
    assert_eq!(key.unwrap(), "stored");
    assert_eq!((looked, created), (2, 1));
}

/// Another creator stored its key between this caller's lookup and its own
/// add, which failed: the caller uses the stored key rather than failing.
#[test]
fn a_failed_create_uses_the_key_another_creator_stored() {
    let (key, looked, created) = run(vec![Ok(None), Ok(Some("theirs"))], Err(Error::Missing));
    assert_eq!(key.unwrap(), "theirs");
    assert_eq!((looked, created), (2, 1));
}

#[test]
fn a_failed_create_with_nothing_stored_reports_why_it_failed() {
    let (key, _, created) = run(vec![Ok(None), Ok(None)], Err(Error::PublicKey));
    assert!(matches!(key, Err(Error::PublicKey)));
    assert_eq!(created, 1);
}

#[test]
fn a_created_key_the_keychain_does_not_hold_is_never_used() {
    let (key, _, created) = run(vec![Ok(None), Ok(None)], Ok(()));
    assert!(matches!(key, Err(Error::Missing)));
    assert_eq!(created, 1);
}

/// The device locked between creating the key and reading it back: the
/// caller waits for an unlock, and its next call finds the stored key.
#[test]
fn a_lock_after_creating_is_transient() {
    let (key, _, created) = run(vec![Ok(None), Err(Error::Locked)], Ok(()));
    assert!(matches!(key, Err(Error::Locked)));
    assert_eq!(created, 1);
}
