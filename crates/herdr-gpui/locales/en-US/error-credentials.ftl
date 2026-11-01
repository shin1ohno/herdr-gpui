# Failures reading or saving a sign-in (GitHub, a cloud provider) in the
# system keyring or the private credential file. Keep config keys and
# values (allow_plaintext_credentials = true, [github], [coder]) and the
# environment variables GH_TOKEN and GITHUB_TOKEN as they are.

error-credentials-encoding = Invalid GitHub credential encoding.
error-credentials-io = Cannot access private credential file.
# macOS only.
error-credentials-keychain-read = Cannot read GitHub Keychain entry. Unlock your login Keychain or set GH_TOKEN.
# macOS only.
error-credentials-keychain-write = GitHub Keychain update failed. Unlock your login Keychain and try again.
# 0600 is the file's Unix permissions: only its owner can read and write it.
error-credentials-permissions = Cannot access private credential file. Require an owned directory and regular 0600 file; symlinks are rejected.
error-credentials-policy = No secure credential store configured. Explicitly opt in with allow_plaintext_credentials = true in the [github] or [coder] table, or use GH_TOKEN / GITHUB_TOKEN for GitHub.
# Linux only.
error-credentials-secret-service-read = Cannot read GitHub sign-in from the desktop keyring. Unlock your keyring or set GH_TOKEN.
# Linux only. Secret Service is the desktop keyring's protocol; GNOME
# Keyring, KWallet, and KeePassXC are programs that provide it.
error-credentials-secret-service-unavailable = No desktop keyring (Secret Service) is running. Start GNOME Keyring, KWallet, or KeePassXC, or set [github] allow_plaintext_credentials = true, or use GH_TOKEN / GITHUB_TOKEN.
# Linux only.
error-credentials-secret-service-write = GitHub keyring update failed. Unlock your desktop keyring and try again.
error-credentials-unsupported = No GitHub credential store is available on this platform. Use GH_TOKEN / GITHUB_TOKEN.

# Windows only.
error-credentials-credential-manager-read = Cannot read a saved sign-in from Windows Credential Manager. Sign in again, or set GH_TOKEN for GitHub.
# Windows only.
error-credentials-credential-manager-write = Windows Credential Manager update failed. Try again.

# iPadOS only. The device key is the key this iPad signs SSH logins with; it
# never leaves the iPad's Keychain.
error-credentials-device-key-access-control = The device key's access control could not be set up.
# iPadOS only.
error-credentials-device-key-create = The device key could not be created.
# iPadOS only.
error-credentials-device-key-item = The Keychain item under the device key's label is not a key.
# iPadOS only.
error-credentials-device-key-keychain = The device key could not be read from or removed from the Keychain.
# iPadOS only.
error-credentials-device-key-locked = The iPad is locked. Unlock it and try again.
# iPadOS only.
error-credentials-device-key-malformed-signature = The device key returned a malformed signature.
# iPadOS only.
error-credentials-device-key-missing = The device key was not in the Keychain after it was created. Try again.
# iPadOS only. P-256 is the key's elliptic curve; keep it as it is.
error-credentials-device-key-public-key = The device key has no P-256 public key.
# iPadOS only.
error-credentials-device-key-sign = The device key could not sign.

# iPadOS only.
error-credentials-ios-keychain-locked = The iPad is locked. Unlock it to use saved sign-ins.
# iPadOS only.
error-credentials-ios-keychain-read = Cannot read a saved sign-in from the Keychain. Sign in again.
# iPadOS only.
error-credentials-ios-keychain-value = The Keychain returned a saved sign-in that is not data. Sign in again.
# iPadOS only.
error-credentials-ios-keychain-write = Saving the sign-in to the Keychain failed. Try again.
