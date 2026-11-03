use super::*;
use herdr_backend::Generation;

#[test]
fn an_unavailable_theme_preserves_connection_and_language_settings() -> anyhow::Result<()> {
    crate::i18n::show(&herdr_i18n::Locale::FALLBACK);
    let profile = "0123456789abcdef0123456789abcdef";
    let language: herdr_i18n::Language = "ja".parse()?;
    let config = Config {
        devices: std::collections::BTreeMap::from([(
            profile.into(),
            crate::config::DeviceSettings {
                generation: Some(Generation::Herdr2),
                ..Default::default()
            },
        )]),
        language: language.clone(),
        theme: "../invalid".into(),
        ..Default::default()
    };
    let appearance = InitialAppearance::load(|| Ok(config), false);
    assert_eq!(
        appearance
            .config
            .device_generation(&format!("ssh:{profile}")),
        Some(Generation::Herdr2)
    );
    assert_eq!(appearance.config.language, language);
    assert_eq!(appearance.config.theme, "../invalid");
    assert_eq!(
        appearance.error.as_deref(),
        Some("Load GUI theme: \u{2068}theme must be a name, absolute path, or ~/ path\u{2069}")
    );
    Ok(())
}

#[test]
fn an_unavailable_theme_localizes_the_startup_message_and_its_cause() {
    crate::i18n::show(&herdr_i18n::Pseudo::Accented.locale());
    let appearance = InitialAppearance::load(
        || {
            Ok(Config {
                theme: "../invalid".into(),
                ..Default::default()
            })
        },
        false,
    );
    crate::i18n::show(&herdr_i18n::Locale::FALLBACK);
    let shown = appearance.error.as_deref().unwrap_or_default();
    assert!(herdr_i18n::pseudo::is_marked(shown), "{shown}");
    assert!(!shown.contains("Load GUI theme"), "{shown}");
    assert!(!shown.contains("theme must be a name"), "{shown}");
}
