//! Replacement ordering shared by the real Keychain and host-side fakes.
//! Every lookup validates the stored key before it can be promoted.

use super::Error;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Slot {
    // keep-sorted start
    Active,
    Retired,
    Staged,
    // keep-sorted end
}

#[cfg(target_os = "ios")]
pub(super) struct Tags<'a> {
    active: &'a str,
    retired: String,
    staged: String,
}

#[cfg(target_os = "ios")]
impl<'a> Tags<'a> {
    pub(super) fn new(label: &'a str) -> Self {
        Self {
            active: label,
            retired: format!("{label}.retired"),
            staged: format!("{label}.replacement"),
        }
    }

    pub(super) fn tag(&self, slot: Slot) -> &str {
        match slot {
            Slot::Active => self.active,
            Slot::Retired => &self.retired,
            Slot::Staged => &self.staged,
        }
    }
}

/// A label move can be interrupted by a lock or process exit. Prefer the
/// committed active item, then the old item, and finally a staged first key.
/// A failed lookup (especially `Locked`) never falls through to creation.
pub(super) fn recover<K>(
    mut find: impl FnMut(Slot) -> Result<Option<K>, Error>,
    mut retag: impl FnMut(Slot, Slot) -> Result<(), Error>,
) -> Result<Option<K>, Error> {
    for slot in [Slot::Active, Slot::Retired, Slot::Staged] {
        if let Some(key) = find(slot)? {
            if slot != Slot::Active {
                retag(slot, Slot::Active)?;
            }
            return Ok(Some(key));
        }
    }
    Ok(None)
}

/// The old key remains stored until the new key is stored and validated.
/// The sole identity switch is the promotion to `Active`: nothing fallible
/// after it can turn a completed replacement into an apparent failure.
pub(super) fn replace<K>(
    mut find: impl FnMut(Slot) -> Result<Option<K>, Error>,
    create: impl FnOnce() -> Result<(), Error>,
    mut retag: impl FnMut(Slot, Slot) -> Result<(), Error>,
    mut delete: impl FnMut(Slot) -> Result<(), Error>,
) -> Result<K, Error> {
    let old = recover(&mut find, &mut retag)?;
    // A previous interrupted attempt can leave a staged key; a completed
    // attempt can leave a retired one. Clear both before creating another,
    // bounding storage to the active key and one replacement at a time.
    delete(Slot::Staged)?;
    delete(Slot::Retired)?;
    create()?;
    let new = find(Slot::Staged)?.ok_or(Error::Missing)?;
    if old.is_some() {
        retag(Slot::Active, Slot::Retired)?;
    }
    if let Err(error) = retag(Slot::Staged, Slot::Active) {
        if old.is_some() {
            // A locked rollback leaves the old item in Retired. `recover`
            // restores it before a future caller could create another key.
            let _ = retag(Slot::Retired, Slot::Active);
        }
        return Err(error);
    }
    if let Err(error) = delete(Slot::Retired) {
        tracing::warn!(%error, "Replaced the device key; retired key cleanup will be retried");
    }
    Ok(new)
}
