//! The iPad's Keychain as the keyring for saved sign-ins. Each entry is a
//! generic password that only this device can read, and only while it is
//! unlocked (`kSecAttrAccessibleWhenUnlockedThisDeviceOnly`): it is never
//! synchronized and never restored onto another device, and it is in no
//! shared access group. iOS has no plaintext fallback (docs/ios.md).

use crate::{
    Result, credential_bytes,
    sec_item::{self, Name, Query},
};
use core_foundation::{data::CFData, string::CFString};
use secrecy::{ExposeSecret, SecretString};
use security_framework_sys::base::errSecDuplicateItem;

mod error;
pub use error::Error;

/// The entry `account` under `service`. Only a non-synchronizable item
/// matches; the iPad never saves another kind.
fn entry(service: &str, account: &str) -> Query {
    Query::default()
        .constant(Name::Class, Name::ClassGenericPassword)
        .set(Name::Service, CFString::new(service))
        .set(Name::Account, CFString::new(account))
        .flag(Name::Synchronizable, false)
}

pub(crate) fn read(service: &str, account: &str) -> Result<Option<SecretString>> {
    let query = entry(service, account).flag(Name::ReturnData, true);
    let found = sec_item::copy_matching(&query)
        .map_err(|status| Error::from_status(status, Error::Read))?;
    let Some(found) = found else {
        return Ok(None);
    };
    let data = found.downcast_into::<CFData>().ok_or(Error::Value)?;
    credential_bytes(data.bytes().to_vec()).map(Some)
}

/// Saves `token`, or removes the entry when it is `None`; removing an entry
/// that does not exist succeeds.
///
/// A new entry is added with its access class. An existing one has only its
/// value replaced, and only when it already has that class: the update's
/// query names it, so an item kept any other way fails the save rather than
/// keeping a weaker class with a new secret in it.
pub(crate) fn save(service: &str, account: &str, token: Option<&SecretString>) -> Result<()> {
    let write = |status| Error::from_status(status, Error::Write);
    let Some(token) = token else {
        return Ok(sec_item::delete(&entry(service, account)).map_err(write)?);
    };
    let value = CFData::from_buffer(token.expose_secret().as_bytes());
    let protected =
        entry(service, account).constant(Name::Accessible, Name::WhenUnlockedThisDeviceOnly);
    match sec_item::add(&protected.clone().set(Name::ValueData, value.clone())) {
        Err(status) if status.code() == errSecDuplicateItem => {
            let change = Query::default().set(Name::ValueData, value);
            Ok(sec_item::update(&protected, &change).map_err(write)?)
        }
        added => Ok(added.map_err(write)?),
    }
}

#[cfg(test)]
mod tests;
