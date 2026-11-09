use super::*;
use sheprd_backend::Generation;

const ID: &str = "0123456789abcdef0123456789abcdef";

#[test]
fn device_generation_is_an_explicit_choice_and_preserves_other_settings() -> anyhow::Result<()> {
    let directory = TempDirectory::new()?;
    let path = directory.0.join("config-gpui.toml");
    fs::write(&path, format!("# keep\n[devices.{ID}]\ntheme = 'Nord'\n"))?;
    let before = Config::parse(&fs::read_to_string(&path)?)?;
    assert_eq!(before.device_generation(&format!("ssh:{ID}")), None);
    Config::save_device_generation_path(ID, Generation::Herdr2, &path)?;
    let text = fs::read_to_string(&path)?;
    assert!(text.starts_with("# keep\n"));
    let config = Config::parse(&text)?;
    assert_eq!(
        config.device_generation(&format!("ssh:{ID}")),
        Some(Generation::Herdr2)
    );
    assert_eq!(config.device_theme(&format!("ssh:{ID}")), Some("Nord"));
    assert_eq!(config.device_generation("local"), None);
    Config::save_device_generation_path(ID, Generation::Herdr1, &path)?;
    let config = Config::parse(&fs::read_to_string(&path)?)?;
    assert_eq!(
        config.device_generation(&format!("ssh:{ID}")),
        Some(Generation::Herdr1)
    );
    assert!(config.unknown_keys.is_empty());
    Ok(())
}
