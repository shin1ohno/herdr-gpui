use super::*;
use anyhow::Context as _;
use std::fs;

fn paths(root: &Path) -> io::Result<(PathBuf, PathBuf)> {
    let legacy = root.join(".config/herdr/config-gpui.toml");
    let current = root.join("Library/Application Support/herdr/config-gpui.toml");
    for path in [&legacy, &current] {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
    }
    Ok((legacy, current))
}

fn prepare(path: &Path, legacy: &Path) -> Result<(File, PathBuf)> {
    Config::prepare_files_with(path, || {
        migrate_or_warn(path, legacy);
        Ok(())
    })
}

fn check_recovery(
    original: &str,
    personal: Option<&str>,
    check_error: impl Fn(&crate::Error),
) -> anyhow::Result<()> {
    let root = tempfile::tempdir()?;
    let (legacy, current) = paths(root.path())?;
    let old_local = legacy.with_extension("local.toml");
    let local = current.with_extension("local.toml");
    let daemon = root.path().join("absent.toml");
    fs::write(&legacy, original)?;
    if let Some(personal) = personal {
        fs::write(&old_local, personal)?;
    }
    let error = migrate(&current, &legacy)
        .err()
        .context("direct migration accepted invalid legacy settings")?;
    check_error(&error);
    let diagnostic = error.to_string();
    assert!(!current.exists());
    assert!(!local.exists());

    drop(prepare(&current, &legacy)?);
    assert_eq!(fs::read_to_string(&current)?, super::super::DEFAULT_CONFIG);
    assert_eq!(fs::read_to_string(&local)?, LOCAL_CONFIG);
    assert_eq!(startup_path(&current, &legacy)?, current);
    let warning = warning_for(&current).context("lost migration warning")?;
    check_error(&warning.error);
    // A save is allowed immediately after recovery, before any config load
    // has claimed the warning for presentation.
    Config::default().save_theme_at("Nord", &current)?;
    drop(prepare(&current, &legacy)?);
    assert_eq!(Config::default().diagnostic(), None);
    let loaded = Config::load_path(&current, &daemon)?;
    let cloned = loaded.clone();
    let next = Config::load_path(&current, &daemon)?;
    assert_eq!(loaded.theme, "Nord");
    let unrelated = root.path().join("unrelated/config-gpui.toml");
    assert_eq!(Config::load_path(&unrelated, &daemon)?.diagnostic(), None);
    assert_eq!(cloned.diagnostic(), Some(diagnostic));
    assert_eq!(loaded.diagnostic(), None);
    assert_eq!(next.diagnostic(), None);
    assert_eq!(Config::load_path(&current, &daemon)?.diagnostic(), None);

    assert_eq!(fs::read_to_string(&legacy)?, original);
    assert_eq!(old_local.exists(), personal.is_some());
    if let Some(personal) = personal {
        assert_eq!(fs::read_to_string(&old_local)?, personal);
    }
    Ok(())
}

#[test]
fn conflicting_legacy_settings_allow_saves_and_warn_once() -> anyhow::Result<()> {
    check_recovery("theme = 'Nord'\n", Some("theme = 'Dracula'\n"), |error| {
        assert!(matches!(
            error,
            crate::Error::Settings(Error::MigrationConflict { .. })
        ));
    })
}

#[test]
fn invalid_legacy_settings_allow_saves_and_warn_once() -> anyhow::Result<()> {
    check_recovery("theme = [", None, |error| {
        assert!(matches!(error, crate::Error::Path { source, .. }
            if matches!(source.as_ref(), crate::Error::ConfigFile { .. })));
    })
}

#[test]
fn oversized_legacy_settings_allow_saves_and_warn_once() -> anyhow::Result<()> {
    check_recovery(&"#".repeat(MAX_CONFIG_BYTES as usize + 1), None, |error| {
        assert!(matches!(error, crate::Error::Path { source, .. }
            if matches!(source.as_ref(), crate::Error::Io(error)
                if error.kind() == ErrorKind::FileTooLarge)));
    })
}

#[test]
fn valid_legacy_settings_still_migrate_without_a_warning() -> anyhow::Result<()> {
    let root = tempfile::tempdir()?;
    let (legacy, current) = paths(root.path())?;
    let original = "# keep this comment\r\ntheme = 'Dracula'\r\n";
    fs::write(&legacy, original)?;
    drop(prepare(&current, &legacy)?);
    assert_eq!(
        fs::read_to_string(current.with_extension("local.toml"))?,
        original
    );
    assert_eq!(fs::read_to_string(&legacy)?, original);
    assert!(warning_for(&current).is_none());
    let config = Config::load_path(&current, &root.path().join("absent.toml"))?;
    assert_eq!(config.theme, "Dracula");
    assert_eq!(config.diagnostic(), None);
    Ok(())
}

#[test]
fn destination_errors_are_not_downgraded_to_migration_warnings() -> anyhow::Result<()> {
    let root = tempfile::tempdir()?;
    let (legacy, current) = paths(root.path())?;
    let local = current.with_extension("local.toml");
    fs::write(&legacy, "theme = 'Nord'\n")?;
    fs::create_dir(&local)?;
    for result in [
        prepare(&current, &legacy).map(drop),
        Config::default().save_theme_at("Dracula", &current),
    ] {
        let error = result.err().context("ignored invalid destination")?;
        assert!(matches!(error, crate::Error::Path { path, source }
            if path == local && matches!(source.as_ref(), crate::Error::Io(_))));
    }
    assert!(local.is_dir());
    assert!(!current.exists());
    assert!(warning_for(&current).is_none());
    assert_eq!(fs::read_to_string(&legacy)?, "theme = 'Nord'\n");
    Ok(())
}

#[test]
fn migration_warning_registry_is_bounded_deduplicated_and_redacts_causes() -> anyhow::Result<()> {
    let mut warnings = MigrationWarnings::new();
    for index in 0..=MAX_MIGRATION_WARNINGS {
        assert!(
            warnings.record(
                Path::new(&format!("destination-{index}")),
                crate::Error::from(io::Error::other("private parser excerpt"))
                    .at_path(Path::new("private source path")),
            )
        );
    }
    assert_eq!(warnings.notices.len(), MAX_MIGRATION_WARNINGS);
    assert!(warnings.get(Path::new("destination-0")).is_none());
    let last = PathBuf::from(format!("destination-{MAX_MIGRATION_WARNINGS}"));
    assert!(!warnings.record(&last, crate::Error::MissingHome));
    let warning = warnings.get(&last).context("lost newest warning")?;
    assert_eq!(warning_kind(&warning.error), "Io");
    let debug = format!("{warning:?}");
    assert!(!debug.contains("private parser excerpt"));
    assert!(!debug.contains("private source path"));
    assert!(warning.diagnostic().is_some());
    assert_eq!(warning.diagnostic(), None);
    Ok(())
}

#[test]
fn concurrent_configs_present_a_migration_warning_only_once() -> anyhow::Result<()> {
    let warning = Arc::new(MigrationWarning {
        error: crate::Error::MissingHome,
        shown: AtomicBool::new(false),
    });
    let shown = std::thread::scope(|scope| {
        let handles: Vec<_> = (0..4)
            .map(|_| scope.spawn(|| warning.diagnostic()))
            .collect();
        handles.into_iter().try_fold(0, |shown, handle| {
            let diagnostic = handle
                .join()
                .map_err(|_| anyhow::anyhow!("diagnostic worker panicked"))?;
            Ok::<_, anyhow::Error>(shown + usize::from(diagnostic.is_some()))
        })
    })?;
    assert_eq!(shown, 1);
    Ok(())
}

#[test]
fn migration_notice_precedes_other_config_diagnostics() -> anyhow::Result<()> {
    let warning = Arc::new(MigrationWarning {
        error: crate::Error::MissingHome,
        shown: AtomicBool::new(false),
    });
    let diagnostic = Config {
        migration_warning: Some(warning),
        missing_fonts: vec!["Unknown font".into()],
        unknown_keys: vec!["future_setting".into()],
        ..Config::default()
    }
    .diagnostic()
    .context("lost diagnostics")?;
    assert_eq!(
        diagnostic.lines().next(),
        Some(crate::Error::MissingHome.to_string().as_str())
    );
    assert!(diagnostic.contains("future_setting"));
    assert!(diagnostic.contains("Unknown font"));
    Ok(())
}
