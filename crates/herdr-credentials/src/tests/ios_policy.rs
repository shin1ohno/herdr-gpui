use super::*;

/// The iPad keeps saved sign-ins in its Keychain whatever the opt-in says
/// (lib.rs also asserts this when compiling), and a caller that names the
/// file anyway writes and reads nothing.
#[test]
fn the_ipad_never_uses_the_plaintext_file() {
    assert_eq!(Store::for_policy(true), Store::Keyring);
    assert_eq!(Store::for_policy(false), Store::Keyring);
    let accept: fn(&SecretString) -> Result<()> = |_| Ok(());
    let directory = tempfile::tempdir().unwrap();
    let name = c"github-credentials";
    assert!(matches!(
        file::store_checked(directory.path(), name, Some(&"token".into()), true, accept),
        Err(Error::Unsupported)
    ));
    assert!(
        std::fs::read_dir(directory.path())
            .unwrap()
            .next()
            .is_none()
    );
    // A file some other build left is never read.
    use std::os::unix::fs::PermissionsExt;
    let left = directory.path().join("github-credentials");
    std::fs::write(&left, "token").unwrap();
    std::fs::set_permissions(&left, std::fs::Permissions::from_mode(0o600)).unwrap();
    assert!(
        file::read_checked(directory.path(), name, accept)
            .unwrap()
            .is_none()
    );
    assert!(file::store_checked(directory.path(), name, None, false, accept).is_ok());
}
