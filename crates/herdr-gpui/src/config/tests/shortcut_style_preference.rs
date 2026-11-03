use super::*;
use crate::config::preferences::Preference;

#[test]
fn shortcut_style_preference_writes_the_top_level_key() -> anyhow::Result<()> {
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("local.toml");
    fs::write(
        &path,
        "# personal\nfuture = 'keep'\nshortcut_style = 'ipad' # style\n[keybindings]\nnew_tab = 'cmd-alt-t'\n[chat]\nlayout = 'bubbles'\n",
    )?;
    for &style in ShortcutStyle::ALL {
        Config::save_preference_path(Preference::ShortcutStyle(style), &path)?;
        let text = fs::read_to_string(&path)?;
        let table: toml::Table = toml::from_str(&text)?;
        assert_eq!(table["shortcut_style"].as_str(), Some(style.name()));
        assert_eq!(table["future"].as_str(), Some("keep"));
        assert_eq!(table["keybindings"]["new_tab"].as_str(), Some("cmd-alt-t"));
        assert!(table["keybindings"].get("shortcut_style").is_none());
        assert!(table["chat"].get("shortcut_style").is_none());
        assert!(text.contains("# personal") && text.contains("# style"));
        assert_eq!(Config::parse(&text)?.shortcut_style, style);
    }
    Ok(())
}

#[test]
fn shortcut_style_preference_creates_no_table() -> anyhow::Result<()> {
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("new.toml");
    Config::save_preference_path(Preference::ShortcutStyle(ShortcutStyle::Ipad), &path)?;
    let text = fs::read_to_string(path)?;
    let table: toml::Table = toml::from_str(&text)?;
    assert_eq!(table["shortcut_style"].as_str(), Some("ipad"));
    assert!(table.values().all(|value| !value.is_table()));
    Ok(())
}

#[test]
fn keyboard_device_preference_preserves_the_other_device_settings() -> anyhow::Result<()> {
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("local.toml");
    let profile = "0123456789abcdef0123456789abcdef";
    fs::write(
        &path,
        format!("shortcut_style = 'ipad'\n[devices.{profile}]\ntheme = 'Nord'\n"),
    )?;
    Config::save_preference_path(
        Preference::DeviceKeybindings {
            profile: profile.into(),
            source: KeybindingSource::Server,
        },
        &path,
    )?;
    let text = fs::read_to_string(&path)?;
    let config = Config::parse(&text)?;
    assert_eq!(config.shortcut_style, ShortcutStyle::Ipad);
    assert_eq!(
        config.devices[profile].keybindings,
        KeybindingSource::Server
    );
    assert_eq!(config.devices[profile].theme.as_deref(), Some("Nord"));
    Config::save_preference_path(
        Preference::DeviceKeybindings {
            profile: profile.into(),
            source: KeybindingSource::Local,
        },
        &path,
    )?;
    let table: toml::Table = toml::from_str(&fs::read_to_string(path)?)?;
    assert!(table["devices"][profile].get("keybindings").is_none());
    assert_eq!(table["devices"][profile]["theme"].as_str(), Some("Nord"));
    Ok(())
}

#[test]
fn keyboard_device_preference_keeps_one_path_and_the_typed_cause() -> anyhow::Result<()> {
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("local.toml");
    fs::write(&path, "devices = false\n")?;
    let Err(error) = Config::save_preference_path(
        Preference::DeviceKeybindings {
            profile: "0123456789abcdef0123456789abcdef".into(),
            source: KeybindingSource::Server,
        },
        &path,
    ) else {
        anyhow::bail!("invalid devices table was accepted");
    };
    let crate::Error::Path {
        path: failed,
        source,
    } = error
    else {
        panic!("missing path context");
    };
    assert_eq!(failed, path);
    assert!(matches!(
        *source,
        crate::Error::Settings(Error::InvalidDevicesTable)
    ));
    assert_eq!(fs::read_to_string(path)?, "devices = false\n");
    Ok(())
}
