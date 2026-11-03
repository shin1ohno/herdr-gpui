//! Keep early iPad builds' overrides when moving configuration into Library.
//! The startup read never writes; migration runs under the background loader's
//! existing config lock and retains the original files.

use super::{Config, Daemon, Error, LOCAL_CONFIG, MANAGED_HEADER};
use crate::{Result, i18n::fl};
use std::{
    collections::VecDeque,
    fmt,
    fs::File,
    io::{self, ErrorKind, Read, Write},
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
};

const MAX_CONFIG_BYTES: u64 = 1 << 20;
const MAX_MIGRATION_WARNINGS: usize = 16;
static MIGRATION_WARNINGS: Mutex<MigrationWarnings> = Mutex::new(MigrationWarnings::new());

pub(crate) struct MigrationWarning {
    error: crate::Error,
    shown: AtomicBool,
}

impl fmt::Debug for MigrationWarning {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Config's Debug must not reveal paths or parser excerpts from the
        // preserved legacy files. The cause is formatted only for the user.
        // i18n: data
        f.debug_tuple("MigrationWarning")
            .field(&warning_kind(&self.error))
            .finish()
    }
}

impl MigrationWarning {
    pub(super) fn diagnostic(&self) -> Option<String> {
        (!self.shown.swap(true, Ordering::Relaxed)).then(|| self.error.to_string())
    }
}

pub(super) fn append_migration_warning(config: &Config, lines: &mut Vec<String>) {
    if let Some(warning) = config
        .migration_warning
        .as_ref()
        .and_then(|warning| warning.diagnostic())
    {
        lines.push(warning);
    }
}

pub(super) fn with_migration_warning(mut config: Config, path: &Path) -> Config {
    config.migration_warning = warning_for(path);
    config
}

#[derive(Debug)]
struct MigrationWarnings {
    notices: VecDeque<(PathBuf, Arc<MigrationWarning>)>,
}

impl MigrationWarnings {
    const fn new() -> Self {
        Self {
            notices: VecDeque::new(),
        }
    }

    fn record(&mut self, path: &Path, error: crate::Error) -> bool {
        if self.notices.iter().any(|(saved, _)| saved == path) {
            return false;
        }
        if self.notices.len() == MAX_MIGRATION_WARNINGS {
            self.notices.pop_front();
        }
        self.notices.push_back((
            path.to_owned(),
            Arc::new(MigrationWarning {
                error,
                shown: AtomicBool::new(false),
            }),
        ));
        true
    }

    fn get(&self, path: &Path) -> Option<Arc<MigrationWarning>> {
        self.notices
            .iter()
            .find_map(|(saved, warning)| (saved == path).then(|| warning.clone()))
    }
}

fn warning_kind(error: &crate::Error) -> &'static str {
    match error {
        crate::Error::Path { source, .. } => warning_kind(source),
        crate::Error::Settings(Error::MigrationConflict { .. }) => "MigrationConflict", // i18n: data
        crate::Error::Settings(_) => "Settings", // i18n: data
        crate::Error::ConfigFile { .. } => "ConfigFile", // i18n: data
        crate::Error::Toml(_) => "Toml",         // i18n: data
        crate::Error::Io(error) if error.kind() == ErrorKind::FileTooLarge => "FileTooLarge", // i18n: data
        crate::Error::Io(_) => "Io", // i18n: data
        _ => "Other",                // i18n: data
    }
}

/// A save can encounter migration before a load does. Keep its typed notice
/// for the matching destination until a loaded config presents it once.
pub(super) fn warning_for(path: &Path) -> Option<Arc<MigrationWarning>> {
    MIGRATION_WARNINGS
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .get(path)
}

pub(super) fn migrate_or_warn(path: &Path, legacy: &Path) {
    if let Err(error) = migrate(path, legacy) {
        let reason = warning_kind(&error);
        let recorded = MIGRATION_WARNINGS
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .record(path, error);
        if recorded {
            tracing::warn!(
                category = "config_migration",
                reason,
                "Could not migrate legacy config"
            );
        }
    }
}

/// Bound the read itself, including files that grow after they are opened.
pub(super) fn read_text(path: &Path) -> io::Result<String> {
    let mut text = String::new();
    File::open(path)?
        .take(MAX_CONFIG_BYTES + 1)
        .read_to_string(&mut text)?;
    if text.len() as u64 > MAX_CONFIG_BYTES {
        return Err(io::Error::new(
            ErrorKind::FileTooLarge,
            fl!("error-config-size"),
        ));
    }
    Ok(text)
}

#[cfg(target_os = "ios")]
pub(super) fn legacy_path() -> Result<PathBuf> {
    Ok(super::home()?.join(".config/herdr/config-gpui.toml"))
}

fn has_config(path: &Path) -> Result<bool> {
    for candidate in [path.to_owned(), path.with_extension("local.toml")] {
        if candidate
            .try_exists()
            .map_err(|error| crate::Error::from(error).at_path(&candidate))?
        {
            return Ok(true);
        }
    }
    Ok(false)
}

pub(super) fn startup_path(path: &Path, legacy: &Path) -> Result<PathBuf> {
    if !has_config(path)? && has_config(legacy)? {
        return Ok(legacy.to_owned());
    }
    Ok(path.to_owned())
}

/// Call only while holding the destination's config lock. Existing destination
/// settings take precedence, including a migration interrupted after publication.
pub(super) fn migrate(path: &Path, legacy: &Path) -> Result<()> {
    if has_config(path)? || !has_config(legacy)? {
        return Ok(());
    }
    let (text, source) = legacy_overrides(legacy)?;
    Config::parse_over(&text, &Daemon::default()).map_err(|error| error.at_path(&source))?;
    let local = path.with_extension("local.toml");
    let parent = local.parent().unwrap_or(Path::new("."));
    let mut file = tempfile::NamedTempFile::new_in(parent)
        .map_err(|error| crate::Error::from(error).at_path(&local))?;
    file.write_all(text.as_bytes())
        .and_then(|()| file.as_file().sync_all())
        .map_err(|error| crate::Error::from(error).at_path(&local))?;
    match file.persist_noclobber(&local) {
        Ok(_) => {}
        Err(error) if error.error.kind() == ErrorKind::AlreadyExists => return Ok(()),
        Err(error) => return Err(crate::Error::from(error.error).at_path(&local)),
    }
    #[cfg(unix)]
    File::open(parent)
        .and_then(|directory| directory.sync_all())
        .map_err(|error| crate::Error::from(error).at_path(parent))?;
    Ok(())
}

fn legacy_overrides(path: &Path) -> Result<(String, PathBuf)> {
    let read = |path: &Path| match read_text(path) {
        Ok(text) => Ok(Some(text)),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(None),
        Err(error) => Err(crate::Error::from(error).at_path(path)),
    };
    let local = path.with_extension("local.toml");
    let original = read(path)?;
    let personal = original
        .as_deref()
        .filter(|text| text.lines().next() != Some(MANAGED_HEADER));
    match read(&local)? {
        Some(text) if personal.is_some_and(|personal| personal != text) => {
            Err(Error::MigrationConflict {
                original: path.to_owned(),
                local,
            }
            .into())
        }
        Some(text) => Ok((text, local)),
        None => Ok(personal.map_or_else(
            || (LOCAL_CONFIG.to_owned(), local),
            |text| (text.to_owned(), path.to_owned()),
        )),
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
#[path = "ipad_storage/tests/bounded_reads.rs"]
mod bounded_reads;

#[cfg(test)]
#[path = "ipad_storage/tests/en_xa.rs"]
mod en_xa;

#[cfg(test)]
#[path = "ipad_storage/tests/recovery.rs"]
mod recovery;
