use super::super::*;

#[test]
fn ipad_credential_failures_keep_their_categories() {
    let device = CredentialError::DeviceKey(sheprd_credentials::device_key::Error::Locked);
    assert_eq!(kind(&Error::Credentials(device)), "device_key");
    let keychain = CredentialError::IosKeychain(sheprd_credentials::ios_keychain::Error::Locked);
    assert_eq!(kind(&Error::Credentials(keychain)), "ios_keychain");
}
