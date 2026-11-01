use super::*;
use anyhow::Context as _;
use std::{error::Error as _, fs};

fn overrides(bytes: usize) -> String {
    let mut text = "theme = 'Nord'\n#".to_owned();
    text.push_str(&" ".repeat(bytes - text.len()));
    text
}

fn assert_too_large<T>(result: Result<T>, expected_path: &Path) -> anyhow::Result<()> {
    let error = result.err().context("accepted oversized config")?;
    assert!(error.source().and_then(|source| source.source()).is_some());
    assert!(matches!(error, crate::Error::Path { path, source }
            if path == expected_path && matches!(source.as_ref(), crate::Error::Io(error)
                if error.kind() == ErrorKind::FileTooLarge)));
    Ok(())
}

#[test]
fn bounded_reader_checks_the_byte_limit_and_preserves_io_errors() -> anyhow::Result<()> {
    let root = tempfile::tempdir()?;
    let path = root.path().join("config-gpui.local.toml");
    let text = overrides(MAX_CONFIG_BYTES as usize);
    fs::write(&path, &text)?;
    assert_eq!(read_text(&path)?, text);
    fs::write(&path, format!("{text} "))?;
    assert_eq!(
        read_text(&path)
            .err()
            .context("accepted extra byte")?
            .kind(),
        ErrorKind::FileTooLarge
    );
    assert_too_large(
        read_text(&path).map_err(|error| crate::Error::from(error).at_path(&path)),
        &path,
    )?;
    fs::write(&path, [0xff])?;
    assert_eq!(
        read_text(&path)
            .err()
            .context("accepted invalid UTF-8")?
            .kind(),
        ErrorKind::InvalidData
    );
    fs::remove_file(&path)?;
    assert_eq!(
        read_text(&path)
            .err()
            .context("accepted absent file")?
            .kind(),
        ErrorKind::NotFound
    );
    Ok(())
}

#[test]
fn startup_and_background_apply_the_platform_limit_to_local_overrides() -> anyhow::Result<()> {
    for bytes in [MAX_CONFIG_BYTES, MAX_CONFIG_BYTES + 1] {
        let root = tempfile::tempdir()?;
        let path = root.path().join("config-gpui.toml");
        let local = path.with_extension("local.toml");
        let daemon = root.path().join("absent.toml");
        let text = overrides(bytes as usize);
        fs::write(&path, super::super::DEFAULT_CONFIG)?;
        fs::write(&local, &text)?;

        if cfg!(target_os = "ios") && bytes > MAX_CONFIG_BYTES {
            assert_too_large(Config::load_startup_path(&path, &daemon), &local)?;
            assert_too_large(Config::prepare_files(&path), &local)?;
            assert_too_large(Config::load_path(&path, &daemon), &local)?;
        } else {
            assert_eq!(Config::load_startup_path(&path, &daemon)?.theme, "Nord");
            drop(Config::prepare_files(&path)?);
            assert_eq!(Config::load_path(&path, &daemon)?.theme, "Nord");
        }
        assert_eq!(fs::read_to_string(&local)?, text);
        assert_eq!(fs::read_to_string(&path)?, super::super::DEFAULT_CONFIG);
    }
    Ok(())
}

#[test]
fn startup_and_maintenance_apply_the_platform_limit_to_personal_configs() -> anyhow::Result<()> {
    let root = tempfile::tempdir()?;
    let path = root.path().join("config-gpui.toml");
    let local = path.with_extension("local.toml");
    let daemon = root.path().join("absent.toml");
    let text = overrides(MAX_CONFIG_BYTES as usize + 1);
    fs::write(&path, &text)?;
    if cfg!(target_os = "ios") {
        assert_too_large(Config::load_startup_path(&path, &daemon), &path)?;
        assert_too_large(Config::prepare_files(&path), &path)?;
        assert_too_large(Config::load_path(&path, &daemon), &path)?;
        assert_eq!(fs::read_to_string(&path)?, text);
        assert!(!local.exists());
    } else {
        assert_eq!(Config::load_startup_path(&path, &daemon)?.theme, "Nord");
        drop(Config::prepare_files(&path)?);
        assert_eq!(Config::load_path(&path, &daemon)?.theme, "Nord");
        assert_eq!(fs::read_to_string(&local)?, text);
        assert_eq!(fs::read_to_string(&path)?, super::super::DEFAULT_CONFIG);
    }
    Ok(())
}

#[test]
fn maintenance_checks_managed_files_even_with_small_local_overrides() -> anyhow::Result<()> {
    let root = tempfile::tempdir()?;
    let path = root.path().join("config-gpui.toml");
    let local = path.with_extension("local.toml");
    let daemon = root.path().join("absent.toml");
    let text = format!(
        "{MANAGED_HEADER}\n#{}",
        " ".repeat(MAX_CONFIG_BYTES as usize)
    );
    fs::write(&path, &text)?;
    fs::write(&local, "theme = 'Nord'\n")?;
    assert_eq!(Config::load_startup_path(&path, &daemon)?.theme, "Nord");
    if cfg!(target_os = "ios") {
        assert_too_large(Config::prepare_files(&path), &path)?;
        assert_too_large(Config::load_path(&path, &daemon), &path)?;
        assert_eq!(fs::read_to_string(&path)?, text);
    } else {
        drop(Config::prepare_files(&path)?);
        assert_eq!(Config::load_path(&path, &daemon)?.theme, "Nord");
        assert_eq!(fs::read_to_string(&path)?, super::super::DEFAULT_CONFIG);
    }
    assert_eq!(fs::read_to_string(&local)?, "theme = 'Nord'\n");
    Ok(())
}

#[test]
fn settings_edits_apply_the_platform_read_limit_without_overwriting_on_error() -> anyhow::Result<()>
{
    use super::super::{CodeEdit, FontFace, LayoutMode};
    use crate::contrast::Contrast;

    let root = tempfile::tempdir()?;
    let path = root.path().join("config-gpui.local.toml");
    let text = overrides(MAX_CONFIG_BYTES as usize + 1);
    let edits: &[fn(&Path) -> Result<()>] = &[
        |path| Config::default().save_theme_path("Dracula", path),
        |path| Config::save_layout_path(LayoutMode::default(), path),
        |path| Config::save_font_sizes_path(&[(FontFace::Terminal, 18.)], path),
        |path| Config::save_font_family_path(FontFace::Terminal, Some("monospace"), path),
        |path| Config::save_contrast_path(Contrast::default(), path),
        |path| Config::save_show_agents_path(false, path),
        |path| Config::save_usage_visibility_path(false, path),
        |path| Config::save_code_path(&CodeEdit::AcceptLicense, path),
    ];
    for edit in edits {
        fs::write(&path, &text)?;
        let result = edit(&path);
        if cfg!(target_os = "ios") {
            assert_too_large(result, &path)?;
            assert_eq!(fs::read_to_string(&path)?, text);
        } else {
            result?;
        }
    }
    Ok(())
}
