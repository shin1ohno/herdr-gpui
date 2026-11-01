//! Edit the app's saved devices without invoking Herdr's CLI. These operations
//! change only the local catalog and never contact or stop a remote daemon.
use super::*;

/// Remove a saved device. Returns false when it is already absent. Blocking
/// filesystem I/O belongs on a background worker.
pub fn remove_saved_host(development: bool, id: &str) -> Result<bool> {
    remove(&catalog_path(development, |name| env::var_os(name)), id)
}

/// Rename a saved device, retaining its target, session and selection. Returns
/// false if it is absent. Blocking filesystem I/O belongs on a background worker.
pub fn rename_saved_host(development: bool, id: &str, label: &str) -> Result<bool> {
    rename(
        &catalog_path(development, |name| env::var_os(name)),
        id,
        label,
    )
}

fn remove(path: &Path, id: &str) -> Result<bool> {
    edit(path, id, |catalog, index| {
        catalog.ssh.remove(index);
        if catalog.selected_profile.as_deref() == Some(id) {
            catalog.selected_profile = None;
        }
    })
}

fn rename(path: &Path, id: &str, label: &str) -> Result<bool> {
    edit(path, id, |catalog, index| {
        catalog.ssh[index].label = label.to_owned();
    })
}

fn edit(path: &Path, id: &str, update: impl FnOnce(&mut Catalog, usize)) -> Result<bool> {
    if !valid_profile_id(id) {
        return Err(Error::ProfileId);
    }
    let _guard = WRITER.lock().unwrap_or_else(|error| error.into_inner());
    let mut catalog = load_catalog(path)?;
    let Some(index) = catalog.ssh.iter().position(|host| host.id == id) else {
        return Ok(false);
    };
    update(&mut catalog, index);
    write_catalog(path, &catalog)?;
    Ok(true)
}

#[cfg(test)]
mod tests;
