#![allow(clippy::unwrap_used)]
use super::*;

#[cfg(target_os = "ios")]
mod ios_policy;

#[test]
fn keyring_follows_each_platform_policy() {
    assert_eq!(Store::choose(false, true, false), Store::Keyring);
    // The plaintext opt-in wins over a keyring where it is honoured at all, so
    // a Linux desktop without a Secret Service still has a way to save.
    assert_eq!(Store::choose(true, true, false), Store::File);
    // An unsigned macOS development build gets a new code identity on every
    // rebuild, so it uses the private file instead of re-prompting for Keychain.
    assert_eq!(Store::choose(false, false, true), Store::File);
    assert_eq!(Store::choose(true, false, true), Store::File);
    // Everywhere else unencrypted storage stays an explicit opt-in.
    assert_eq!(Store::choose(false, false, false), Store::Environment);
    assert_eq!(Store::choose(true, false, false), Store::File);
    assert_eq!(
        KEYRING,
        cfg!(target_os = "linux")
            || cfg!(windows)
            || cfg!(target_os = "ios")
            || (cfg!(target_os = "macos") && RELEASE_BUILD)
    );
    // Platforms without POSIX ownership and mode bits cannot keep the file
    // private, so opting in must not select it there; the iPad never keeps it.
    assert_eq!(FILE, cfg!(unix) && !cfg!(target_os = "ios"));
    assert_eq!(
        Store::for_policy(false),
        Store::choose(false, KEYRING, FILE_DEFAULT)
    );
}

/// Off POSIX a file cannot be kept private: writing one is refused, and
/// removing one, which cannot exist, succeeds.
#[cfg(not(unix))]
#[test]
fn the_private_file_is_unsupported_off_posix() {
    let accept: fn(&SecretString) -> Result<()> = |_| Ok(());
    let name = c"github-credentials";
    let path = Path::new(".");
    assert!(matches!(
        file::store_checked(path, name, Some(&"token".into()), true, accept),
        Err(Error::Unsupported)
    ));
    assert!(file::store_checked(path, name, None, false, accept).is_ok());
    assert!(file::read_checked(path, name, accept).unwrap().is_none());
}
