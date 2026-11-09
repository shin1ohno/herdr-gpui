//! Reads back what a test stored, to check the attributes Security.framework
//! actually kept rather than the ones this crate asked for.
#![allow(clippy::unwrap_used)]
use super::*;
use core_foundation::{array::CFArray, number::CFNumber};

/// The attributes of every item `query` matches.
pub(crate) fn stored(query: Query) -> Vec<CFDictionary<CFString, CFType>> {
    let query = query
        .flag(Name::ReturnAttributes, true)
        .constant(Name::MatchLimit, Name::MatchLimitAll);
    let Some(found) = copy_matching(&query).unwrap() else {
        return Vec::new();
    };
    let items = found.downcast_into::<CFArray>().unwrap();
    items
        .iter()
        .map(|item| {
            // SAFETY: with `kSecMatchLimitAll` and `kSecReturnAttributes`
            // Security.framework returns an array of attribute dictionaries
            // keyed by `CFString`; the get rule retains each one beyond the
            // array's life.
            #[allow(unsafe_code)]
            unsafe {
                CFDictionary::wrap_under_get_rule((*item).cast())
            }
        })
        .collect()
}

/// Whether the item is readable only on this device, while it is unlocked.
pub(crate) fn device_only(attributes: &CFDictionary<CFString, CFType>) -> bool {
    attributes
        .find(Name::Accessible.string())
        .and_then(|class| class.downcast::<CFString>())
        == Some(Name::WhenUnlockedThisDeviceOnly.string())
}

/// The item's `kSecAttrSynchronizable`, which Security.framework may report
/// as a boolean or a number.
pub(crate) fn synchronizable(attributes: &CFDictionary<CFString, CFType>) -> Option<bool> {
    let value = attributes.find(Name::Synchronizable.string())?;
    value.downcast::<CFBoolean>().map(bool::from).or_else(|| {
        value
            .downcast::<CFNumber>()
            .and_then(|number| number.to_i64())
            .map(|number| number != 0)
    })
}
