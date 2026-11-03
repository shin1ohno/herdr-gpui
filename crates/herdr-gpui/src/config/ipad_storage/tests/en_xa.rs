use super::*;
use crate::i18n::Localize as _;
use anyhow::Context as _;
use herdr_i18n::{Locale, Pseudo};
use std::fs;

#[test]
fn oversized_config_localizes_the_message_and_preserves_the_io_kind() -> anyhow::Result<()> {
    let root = tempfile::tempdir()?;
    let path = root.path().join("config-gpui.local.toml");
    fs::write(&path, "#".repeat(MAX_CONFIG_BYTES as usize + 1))?;
    for locale in [Locale::FALLBACK, Pseudo::Accented.locale()] {
        crate::i18n::show(&locale);
        let result = read_text(&path);
        crate::i18n::show(&Locale::FALLBACK);
        let error = result.err().context("accepted oversized config")?;
        assert_eq!(error.kind(), ErrorKind::FileTooLarge);
        crate::i18n::show(&locale);
        let shown = crate::Error::from(error).at_path(&path).localize();
        crate::i18n::show(&Locale::FALLBACK);
        if locale == Locale::FALLBACK {
            assert!(shown.contains("configuration exceeds 1 MiB"), "{shown}");
        } else {
            assert!(herdr_i18n::pseudo::is_marked(&shown), "{shown}");
            assert!(!shown.contains("configuration exceeds"), "{shown}");
        }
    }
    Ok(())
}
