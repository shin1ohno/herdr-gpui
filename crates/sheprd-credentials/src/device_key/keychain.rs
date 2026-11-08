//! The device key's Keychain item: a P-256 private key under an application
//! tag. On a device it is created in the Secure Enclave, which keeps the
//! private key and signs with it; the simulator has no Secure Enclave, so
//! there the Keychain keeps a software key. Either way the item is readable
//! only on this device while it is unlocked, never synchronized, and in no
//! shared access group.

use super::Error;
use crate::sec_item::{self, Name, Query};
use core_foundation::{data::CFData, number::CFNumber};
use security_framework::{
    access_control::{ProtectionMode, SecAccessControl},
    key::{Algorithm, SecKey},
    passwords::AccessControlOptions,
};

/// Every iPad has a Secure Enclave and the simulator has none. Chosen when
/// building, so a device never falls back to a software key.
const SECURE_ENCLAVE: bool = !cfg!(target_abi = "sim");

/// The private key under `label`, of the kind `create` makes.
pub(super) fn tagged(label: &str) -> Query {
    Query::default()
        .constant(Name::Class, Name::ClassKey)
        .constant(Name::KeyClass, Name::KeyClassPrivate)
        .constant(Name::KeyType, Name::KeyTypeEcPrimeRandom)
        .set(Name::ApplicationTag, CFData::from_buffer(label.as_bytes()))
        .flag(Name::Synchronizable, false)
}

pub(super) fn find(label: &str) -> Result<Option<SecKey>, Error> {
    let query = tagged(label).flag(Name::ReturnRef, true);
    sec_item::copy_matching(&query)
        .map_err(|status| Error::from_status(status, Error::Keychain))?
        .map(|found| found.downcast_into::<SecKey>().ok_or(Error::Item))
        .transpose()
}

/// Creates a key under `label` and stores it. Callers look it up again
/// rather than use it (`create::find_or_create`).
pub(super) fn create(label: &str) -> Result<(), Error> {
    let stored = Query::default()
        .flag(Name::IsPermanent, true)
        .set(Name::ApplicationTag, CFData::from_buffer(label.as_bytes()));
    let parameters = Query::default()
        .constant(Name::KeyType, Name::KeyTypeEcPrimeRandom)
        .set(Name::KeySizeInBits, CFNumber::from(256));
    let (parameters, stored) = if SECURE_ENCLAVE {
        // The access class goes in the access control; an item cannot name
        // both. Private key usage is what lets the enclave sign with it. An
        // enclave key cannot leave the device, so it is never synchronized.
        let access = SecAccessControl::create_with_protection(
            Some(ProtectionMode::AccessibleWhenUnlockedThisDeviceOnly),
            AccessControlOptions::PRIVATE_KEY_USAGE.bits(),
        )
        .map_err(Error::AccessControl)?;
        (
            parameters.constant(Name::TokenId, Name::TokenIdSecureEnclave),
            stored.set(Name::AccessControl, access),
        )
    } else {
        (
            parameters,
            stored
                .constant(Name::Accessible, Name::WhenUnlockedThisDeviceOnly)
                .flag(Name::Synchronizable, false),
        )
    };
    let parameters = parameters.set(Name::PrivateKeyAttrs, stored.dictionary());
    // `SecKey::new` takes `GenerateKeyOptions`, which off macOS leaves the
    // private key's attributes out (security-framework 3.7.0 key.rs:448-452),
    // so the key would be neither stored nor protected.
    #[allow(deprecated)]
    let created = SecKey::generate(parameters.dictionary().to_untyped());
    created.map_err(|error| Error::from_cf(&error, Error::Create))?;
    tracing::info!("Created a new device key");
    Ok(())
}

pub(super) fn delete(label: &str) -> Result<(), Error> {
    sec_item::delete(&tagged(label)).map_err(|status| Error::from_status(status, Error::Keychain))
}

/// Changes only the application tag, preserving the Secure Enclave key and
/// its access control. Callers hold `KEYCHAIN` and keep destination tags empty.
pub(super) fn retag(from: &str, to: &str) -> Result<(), Error> {
    let changes = Query::default().set(Name::ApplicationTag, CFData::from_buffer(to.as_bytes()));
    sec_item::update(&tagged(from), &changes)
        .map_err(|status| Error::from_status(status, Error::Keychain))
}

/// The key's public half as a 65-byte uncompressed SEC1 point.
pub(super) fn public_sec1(key: &SecKey) -> Result<[u8; 65], Error> {
    let public = key.public_key().ok_or(Error::PublicKey)?;
    let encoded = public.external_representation().ok_or(Error::PublicKey)?;
    match <[u8; 65]>::try_from(encoded.bytes()) {
        Ok(point) if point[0] == 0x04 => Ok(point),
        _ => Err(Error::PublicKey),
    }
}

/// ECDSA over SHA-256 of `message`, in the DER form Security.framework
/// returns.
pub(super) fn sign(key: &SecKey, message: &[u8]) -> Result<Vec<u8>, Error> {
    key.create_signature(Algorithm::ECDSASignatureMessageX962SHA256, message)
        .map_err(|error| Error::from_cf(&error, Error::Sign))
}
