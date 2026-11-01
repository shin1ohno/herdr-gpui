use super::*;

#[test]
fn shortcut_styles_parse_serialize_and_override() -> anyhow::Result<()> {
    for (name, style) in [
        ("ipad", ShortcutStyle::Ipad),
        ("herdr-prefix", ShortcutStyle::HerdrPrefix),
    ] {
        let config = Config::parse(&format!("shortcut_style = '{name}'"))?;
        assert_eq!(config.shortcut_style, style);
        assert!(config.unknown_keys.is_empty());
        assert_eq!(serde_json::to_value(style)?, name);
    }
    let config = Config::parse_layers(
        ["shortcut_style = 'herdr-prefix'", "shortcut_style = 'ipad'"],
        &Daemon::default(),
    )?;
    assert_eq!(config.shortcut_style, ShortcutStyle::Ipad);
    for value in ["'other'", "true", "7"] {
        let Err(error) = Config::parse(&format!("shortcut_style = {value}")) else {
            anyhow::bail!("invalid shortcut style was accepted: {value}");
        };
        assert!(matches!(error, crate::Error::Toml(_)));
        assert!(std::error::Error::source(&error).is_some());
    }
    Ok(())
}

#[test]
fn shortcut_style_default_uses_the_target_not_the_layout_guard() -> anyhow::Result<()> {
    let expected = if cfg!(target_os = "ios") {
        ShortcutStyle::Ipad
    } else {
        ShortcutStyle::HerdrPrefix
    };
    assert_eq!(Config::default().shortcut_style, expected);
    let _ipad = crate::ipad_ui::ipad_layout();
    assert_eq!(Config::default().shortcut_style, expected);
    assert_eq!(Config::parse("")?.shortcut_style, expected);
    Ok(())
}
