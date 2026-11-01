use super::*;
use herdr_backend::Generation;

#[test]
fn an_unavailable_theme_preserves_connection_and_language_settings() -> anyhow::Result<()> {
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
    assert!(appearance.error.is_some());
    Ok(())
}
