//! Finding the device key, or creating it when there is none, so that every
//! caller ends with the one key the Keychain holds.

use super::Error;

/// The key `find` returns, created by `create` only when `find` reports
/// none. A failure to look, such as a locked device, is returned as it is
/// and never leads to creating a key. After `create`, whether it succeeded
/// or failed (another creator may have stored a key first), the result is
/// the key a second `find` returns, never one `create` made but the Keychain
/// does not hold.
pub(crate) fn find_or_create<K>(
    mut find: impl FnMut() -> Result<Option<K>, Error>,
    create: impl FnOnce() -> Result<(), Error>,
) -> Result<K, Error> {
    if let Some(key) = find()? {
        return Ok(key);
    }
    let created = create();
    match find()? {
        Some(key) => Ok(key),
        None => Err(created.err().unwrap_or(Error::Missing)),
    }
}
