//! Keychain items on iOS, through `SecItem*` with dictionaries this crate
//! builds. security-framework's builders cannot name an application tag or an
//! access class (`kSecAttrApplicationTag`, `kSecAttrAccessible`), and off
//! macOS its key options drop the private key's attributes, so the device key
//! and saved sign-ins state every attribute here (docs/ios.md, "Client key").
//!
//! `unsafe` reads Security.framework's constant names and calls the four
//! `SecItem*` functions, which no safe wrapper offers for an arbitrary query.
//! Each block states what it relies on.

use core_foundation::{
    base::{CFType, CFTypeRef, OSStatus, TCFType},
    boolean::CFBoolean,
    dictionary::CFDictionary,
    string::{CFString, CFStringRef},
};
use security_framework::base::Error;
#[cfg(test)]
use security_framework_sys::item::{kSecMatchLimit, kSecMatchLimitAll, kSecReturnAttributes};
use security_framework_sys::{
    access_control::kSecAttrAccessibleWhenUnlockedThisDeviceOnly,
    base::{errSecItemNotFound, errSecSuccess},
    item::{
        kSecAttrAccessControl, kSecAttrAccount, kSecAttrIsPermanent, kSecAttrKeyClass,
        kSecAttrKeyClassPrivate, kSecAttrKeySizeInBits, kSecAttrKeyType,
        kSecAttrKeyTypeECSECPrimeRandom, kSecAttrService, kSecAttrSynchronizable, kSecAttrTokenID,
        kSecAttrTokenIDSecureEnclave, kSecClass, kSecClassGenericPassword, kSecClassKey,
        kSecPrivateKeyAttrs, kSecReturnData, kSecReturnRef, kSecValueData,
    },
    keychain_item::{SecItemAdd, SecItemCopyMatching, SecItemDelete, SecItemUpdate},
};

/// `errSecInteractionNotAllowed`: the item's access class keeps it out of
/// reach while the device is locked. Transient; never a reason to replace it.
pub(crate) const LOCKED: OSStatus = -25308;

// Security.framework exports these beside the names security-framework-sys
// declares; the sys crate leaves them out.
#[allow(unsafe_code)]
#[link(name = "Security", kind = "framework")]
unsafe extern "C" {
    static kSecAttrAccessible: CFStringRef;
    static kSecAttrApplicationTag: CFStringRef;
}

/// A Security.framework name: an attribute key or one of its constant values.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Name {
    // keep-sorted start
    AccessControl,
    Accessible,
    Account,
    ApplicationTag,
    Class,
    ClassGenericPassword,
    ClassKey,
    IsPermanent,
    KeyClass,
    KeyClassPrivate,
    KeySizeInBits,
    KeyType,
    KeyTypeEcPrimeRandom,
    #[cfg(test)]
    MatchLimit,
    #[cfg(test)]
    MatchLimitAll,
    PrivateKeyAttrs,
    #[cfg(test)]
    ReturnAttributes,
    ReturnData,
    ReturnRef,
    Service,
    Synchronizable,
    TokenId,
    TokenIdSecureEnclave,
    ValueData,
    WhenUnlockedThisDeviceOnly,
    // keep-sorted end
}

impl Name {
    pub(crate) fn string(self) -> CFString {
        // SAFETY: each is an immutable `CFString` Security.framework exports
        // and keeps alive for the life of the process, so reading the pointer
        // is sound and it is never null; the get rule retains it for the
        // returned value.
        #[allow(unsafe_code)]
        unsafe {
            CFString::wrap_under_get_rule(match self {
                Self::AccessControl => kSecAttrAccessControl,
                Self::Accessible => kSecAttrAccessible,
                Self::Account => kSecAttrAccount,
                Self::ApplicationTag => kSecAttrApplicationTag,
                Self::Class => kSecClass,
                Self::ClassGenericPassword => kSecClassGenericPassword,
                Self::ClassKey => kSecClassKey,
                Self::IsPermanent => kSecAttrIsPermanent,
                Self::KeyClass => kSecAttrKeyClass,
                Self::KeyClassPrivate => kSecAttrKeyClassPrivate,
                Self::KeySizeInBits => kSecAttrKeySizeInBits,
                Self::KeyType => kSecAttrKeyType,
                Self::KeyTypeEcPrimeRandom => kSecAttrKeyTypeECSECPrimeRandom,
                #[cfg(test)]
                Self::MatchLimit => kSecMatchLimit,
                #[cfg(test)]
                Self::MatchLimitAll => kSecMatchLimitAll,
                Self::PrivateKeyAttrs => kSecPrivateKeyAttrs,
                #[cfg(test)]
                Self::ReturnAttributes => kSecReturnAttributes,
                Self::ReturnData => kSecReturnData,
                Self::ReturnRef => kSecReturnRef,
                Self::Service => kSecAttrService,
                Self::Synchronizable => kSecAttrSynchronizable,
                Self::TokenId => kSecAttrTokenID,
                Self::TokenIdSecureEnclave => kSecAttrTokenIDSecureEnclave,
                Self::ValueData => kSecValueData,
                Self::WhenUnlockedThisDeviceOnly => kSecAttrAccessibleWhenUnlockedThisDeviceOnly,
            })
        }
    }
}

/// A query or a set of attributes: the dictionary a `SecItem*` call or
/// `SecKeyCreateRandomKey` takes.
#[derive(Clone, Debug, Default)]
pub(crate) struct Query(Vec<(CFString, CFType)>);

impl Query {
    pub(crate) fn set(mut self, name: Name, value: impl TCFType) -> Self {
        self.0.push((name.string(), value.into_CFType()));
        self
    }

    /// `name` set to another Security.framework name, such as a class.
    pub(crate) fn constant(self, name: Name, value: Name) -> Self {
        self.set(name, value.string())
    }

    pub(crate) fn flag(self, name: Name, value: bool) -> Self {
        self.set(name, CFBoolean::from(value))
    }

    pub(crate) fn dictionary(&self) -> CFDictionary<CFString, CFType> {
        CFDictionary::from_CFType_pairs(&self.0)
    }
}

/// What `query` finds, as it asked for it, or `None` when nothing matches.
pub(crate) fn copy_matching(query: &Query) -> Result<Option<CFType>, Error> {
    let query = query.dictionary();
    let mut found: CFTypeRef = std::ptr::null();
    // SAFETY: `query` is a live dictionary for the whole call and `found` a
    // valid out-pointer. On success it holds null or an object this caller
    // owns (the create rule), which is wrapped exactly once below.
    #[allow(unsafe_code)]
    let status = unsafe { SecItemCopyMatching(query.as_concrete_TypeRef(), &mut found) };
    if status == errSecItemNotFound {
        return Ok(None);
    }
    checked(status)?;
    if found.is_null() {
        return Ok(None);
    }
    // SAFETY: as above; `found` is non-null and owned by this caller.
    #[allow(unsafe_code)]
    let found = unsafe { CFType::wrap_under_create_rule(found) };
    Ok(Some(found))
}

/// Adds an item with `attributes`. It fails with `errSecDuplicateItem` when
/// an item with the same primary key exists, which it leaves untouched.
pub(crate) fn add(attributes: &Query) -> Result<(), Error> {
    let attributes = attributes.dictionary();
    // SAFETY: `attributes` is a live dictionary for the whole call, and a
    // null result pointer asks for nothing back.
    #[allow(unsafe_code)]
    let status = unsafe { SecItemAdd(attributes.as_concrete_TypeRef(), std::ptr::null_mut()) };
    checked(status)
}

/// Sets `changes` on every item `query` matches.
pub(crate) fn update(query: &Query, changes: &Query) -> Result<(), Error> {
    let (query, changes) = (query.dictionary(), changes.dictionary());
    // SAFETY: both dictionaries are live for the whole call, which only
    // reads them.
    #[allow(unsafe_code)]
    let status =
        unsafe { SecItemUpdate(query.as_concrete_TypeRef(), changes.as_concrete_TypeRef()) };
    checked(status)
}

/// Removes every item `query` matches; none matching is not an error.
pub(crate) fn delete(query: &Query) -> Result<(), Error> {
    let query = query.dictionary();
    // SAFETY: `query` is live for the whole call, which only reads it.
    #[allow(unsafe_code)]
    let status = unsafe { SecItemDelete(query.as_concrete_TypeRef()) };
    if status == errSecItemNotFound {
        return Ok(());
    }
    checked(status)
}

fn checked(status: OSStatus) -> Result<(), Error> {
    if status == errSecSuccess {
        Ok(())
    } else {
        Err(Error::from_code(status))
    }
}

#[cfg(test)]
pub(crate) mod tests;
