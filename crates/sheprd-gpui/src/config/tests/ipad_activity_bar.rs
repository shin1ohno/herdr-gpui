use super::*;

#[test]
fn ipad_activity_bar_default_uses_target_and_preserves_explicit_settings() -> anyhow::Result<()> {
    let expected = !cfg!(target_os = "ios");
    assert_eq!(Config::default().sidebar_style.activity_bar, expected);
    assert_eq!(Config::parse("")?.sidebar_style.activity_bar, expected);
    let _ipad = crate::ipad_ui::ipad_layout();
    assert_eq!(Config::default().sidebar_style.activity_bar, expected);
    assert_eq!(
        Config::parse("[sidebar]")?.sidebar_style.activity_bar,
        expected
    );
    for shown in [true, false] {
        let config = Config::parse(&format!("[sidebar]\nactivity_bar = {shown}"))?;
        assert_eq!(config.sidebar_style.activity_bar, shown);
    }
    Ok(())
}
