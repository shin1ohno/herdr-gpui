//! iPad owns its device catalog and starts no provisioning CLI. Adding a host
//! saves the already-probed destination; the connection bridge starts a stopped
//! remote daemon as usual. Removing a host only forgets this local entry.
use super::*;

pub(in crate::menu::devices) fn save(request: &Request, claim: &Claim) -> Result<SaveOutcome> {
    store(request, claim, None)
}

pub(in crate::menu::devices) fn save_core(request: &Request, claim: &Claim) -> Result<SaveOutcome> {
    store(request, claim, Some(herdr_backend::Generation::Herdr2))
}

fn store(
    request: &Request,
    claim: &Claim,
    generation: Option<herdr_backend::Generation>,
) -> Result<SaveOutcome> {
    verify_unsaved(request, claim)?;
    let host = SavedHost {
        id: uuid::Uuid::new_v4().simple().to_string(),
        label: request.label.clone(),
        target: request.target.clone(),
        session: request.session.clone(),
        enabled: true,
    };
    if let Some(generation) = generation {
        // Write the backend choice before publishing the catalog entry, so a
        // concurrent catalog poll cannot connect it through Herdr 1 first.
        crate::config::Config::save_device_generation(&host.id, generation)?;
    }
    let saved = herdr_client::store_saved_host(false, &host)?;
    if saved.id != host.id {
        return Err(Error::Exists(saved.label).into());
    }
    Ok(match generation {
        Some(herdr_backend::Generation::Herdr2) => SaveOutcome::CoreAvailable(host.id),
        _ => SaveOutcome::Ready,
    })
}

pub(in crate::menu::devices) fn remove(id: &str) -> Result<()> {
    if !herdr_client::remove_saved_host(false, id)? {
        return Err(Error::NoProfile.into());
    }
    Ok(())
}

pub(in crate::menu::devices) fn rename(id: &str, label: &str) -> Result<()> {
    if !herdr_client::rename_saved_host(false, id, label)? {
        return Err(Error::NoProfile.into());
    }
    Ok(())
}
