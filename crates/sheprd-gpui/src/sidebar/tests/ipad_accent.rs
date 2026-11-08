use super::*;

#[test]
fn ipad_nord_accent_is_cyan_without_changing_desktop_or_other_themes() {
    let nord = Theme::builtin("Nord").unwrap();
    assert_eq!(crate::ipad_ui::accent(&nord), nord.primary());
    let _ipad = crate::ipad_ui::ipad_layout();
    assert_eq!(crate::ipad_ui::accent(&nord), nord.palette[6]);
    let changed = nord
        .clone()
        .with_contrast(Contrast::High)
        .with_accent(Some(nord.palette[1]));
    assert_eq!(crate::ipad_ui::accent(&changed), nord.palette[6]);
    for name in ["Default", "Dracula", "Catppuccin Latte"] {
        let theme = Theme::builtin(name).unwrap();
        assert_eq!(crate::ipad_ui::accent(&theme), theme.primary(), "{name}");
    }
    let _desktop = crate::ipad_ui::desktop();
    assert_eq!(crate::ipad_ui::accent(&changed), changed.primary());
    assert_eq!(nord.primary(), nord.palette[5]);
}
