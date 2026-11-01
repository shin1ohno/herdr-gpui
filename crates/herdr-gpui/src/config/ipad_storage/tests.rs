use super::*;
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

#[test]
fn startup_reads_old_overrides_without_writes_then_migration_preserves_them() -> anyhow::Result<()>
{
    let root = tempfile::tempdir()?;
    let (legacy, current) = paths(root.path())?;
    let daemon = root.path().join("absent.toml");
    let overrides = "# keep this comment\r\ntheme = 'Nord'\r\n[features]\r\nacp_chats = true\r\n[devices.0123456789abcdef0123456789abcdef]\r\ngeneration = 'herdr2'\r\n";
    fs::write(&legacy, super::super::DEFAULT_CONFIG)?;
    fs::write(legacy.with_extension("local.toml"), overrides)?;
    let startup = startup_path(&current, &legacy)?;
    assert_eq!(startup, legacy);
    let before = Config::load_startup_path(&startup, &daemon)?;
    assert_eq!(before.theme, "Nord");
    assert!(before.features.acp_chats);
    assert_eq!(
        before.device_generation("ssh:0123456789abcdef0123456789abcdef"),
        Some(herdr_backend::Generation::Herdr2)
    );
    assert!(!current.exists());
    assert!(!current.with_extension("local.toml").exists());

    migrate(&current, &legacy)?;
    assert_eq!(
        fs::read_to_string(current.with_extension("local.toml"))?,
        overrides
    );
    assert_eq!(
        fs::read_to_string(legacy.with_extension("local.toml"))?,
        overrides
    );
    assert_eq!(fs::read_to_string(&legacy)?, super::super::DEFAULT_CONFIG);
    assert_eq!(startup_path(&current, &legacy)?, current);
    let after = Config::load_path(&current, &daemon)?;
    assert_eq!(after.theme, before.theme);
    assert_eq!(after.features, before.features);
    assert_eq!(after.devices, before.devices);
    Ok(())
}

#[test]
fn personal_legacy_config_migrates_verbatim_and_can_resume() -> anyhow::Result<()> {
    let root = tempfile::tempdir()?;
    let (legacy, current) = paths(root.path())?;
    let overrides = "# legacy personal settings\ntheme = 'Dracula'\n";
    fs::write(&legacy, overrides)?;
    migrate(&current, &legacy)?;
    migrate(&current, &legacy)?;
    assert_eq!(
        fs::read_to_string(current.with_extension("local.toml"))?,
        overrides
    );
    assert_eq!(fs::read_to_string(&legacy)?, overrides);
    Ok(())
}

#[test]
fn existing_destination_settings_always_win() -> anyhow::Result<()> {
    for local in [false, true] {
        let root = tempfile::tempdir()?;
        let (legacy, current) = paths(root.path())?;
        fs::write(&legacy, "theme = 'Nord'\n")?;
        let destination = if local {
            current.with_extension("local.toml")
        } else {
            current.clone()
        };
        fs::write(&destination, "theme = 'Dracula'\n")?;
        assert_eq!(startup_path(&current, &legacy)?, current);
        migrate(&current, &legacy)?;
        assert_eq!(fs::read_to_string(&destination)?, "theme = 'Dracula'\n");
        assert_eq!(fs::read_to_string(&legacy)?, "theme = 'Nord'\n");
    }
    Ok(())
}

#[test]
fn absent_or_invalid_legacy_settings_never_publish_overrides() -> anyhow::Result<()> {
    let root = tempfile::tempdir()?;
    let (legacy, current) = paths(root.path())?;
    assert_eq!(startup_path(&current, &legacy)?, current);
    migrate(&current, &legacy)?;
    assert!(!current.with_extension("local.toml").exists());
    fs::write(&legacy, "[terminal]\nsize = 2\n")?;
    assert!(matches!(
        migrate(&current, &legacy),
        Err(crate::Error::Path { path, source })
            if path == legacy && matches!(source.as_ref(),
                crate::Error::Settings(Error::InvalidFontSize("terminal")))
    ));
    assert!(!current.with_extension("local.toml").exists());
    Ok(())
}

#[test]
fn conflicting_or_oversized_legacy_settings_are_preserved_without_publication() -> anyhow::Result<()>
{
    let root = tempfile::tempdir()?;
    let (legacy, current) = paths(root.path())?;
    let old_local = legacy.with_extension("local.toml");
    fs::write(&legacy, "theme = 'Nord'\n")?;
    fs::write(&old_local, "theme = 'Dracula'\n")?;
    assert!(matches!(
        migrate(&current, &legacy),
        Err(crate::Error::Settings(Error::MigrationConflict { original, local }))
            if original == legacy && local == old_local
    ));
    assert_eq!(fs::read_to_string(&legacy)?, "theme = 'Nord'\n");
    assert_eq!(fs::read_to_string(&old_local)?, "theme = 'Dracula'\n");
    assert!(!current.with_extension("local.toml").exists());

    fs::remove_file(&old_local)?;
    fs::write(&legacy, "#".repeat(MAX_CONFIG_BYTES as usize + 1))?;
    assert!(matches!(
        migrate(&current, &legacy),
        Err(crate::Error::Path { path, source })
            if path == legacy && matches!(source.as_ref(), crate::Error::Io(error)
                if error.kind() == ErrorKind::FileTooLarge)
    ));
    assert!(!current.with_extension("local.toml").exists());
    assert_eq!(fs::metadata(&legacy)?.len(), MAX_CONFIG_BYTES + 1);
    Ok(())
}
