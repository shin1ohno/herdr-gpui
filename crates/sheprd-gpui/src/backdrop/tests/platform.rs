use super::*;

#[test]
fn a_surface_without_external_transparency_keeps_the_theme_instead_of_blending_with_black() {
    let theme = Theme::default();
    for opacity in [0., 0.2, 0.8, 1.] {
        for blur in [false, true] {
            let config = config::Backdrop {
                opacity,
                blur,
                ..Default::default()
            };
            let fill = Fill::for_surface(&theme, &config, false, false);
            assert!(!fill.translucent());
            assert_eq!(fill.ground(), Hsla::from(rgb(theme.background)));
            assert_eq!(fill.panel(theme.surface), Hsla::from(rgb(theme.surface)));
            assert_eq!(
                fill.base(theme.background),
                Hsla::from(rgb(theme.background))
            );
            assert_eq!(
                appearance_for_surface(&config, false, false),
                WindowBackgroundAppearance::Opaque
            );
            // Rendering policy does not rewrite portable saved settings.
            assert_eq!(config.opacity, opacity);
            assert_eq!(config.blur, blur);
        }
    }
}

#[test]
fn an_available_picture_keeps_its_tint_without_requiring_external_transparency() {
    let theme = Theme::default();
    let config = backdrop(0.35);
    let fill = Fill::for_surface(&theme, &config, true, false);
    assert_eq!(fill.ground().a, 0.35);
    assert!(fill.translucent());
    assert_eq!(
        appearance_for_surface(&config, true, false),
        WindowBackgroundAppearance::Opaque
    );
    // A failed or removed image restores the real theme color.
    assert_eq!(
        Fill::for_surface(&theme, &config, false, false).ground().a,
        1.
    );
}

#[test]
fn desktop_surfaces_keep_opacity_and_native_blur() {
    let theme = Theme::default();
    let mut config = backdrop(0.35);
    assert_eq!(
        Fill::for_surface(&theme, &config, false, true).ground().a,
        0.35
    );
    assert_eq!(
        appearance_for_surface(&config, false, true),
        WindowBackgroundAppearance::Transparent
    );
    config.blur = true;
    assert_eq!(
        appearance_for_surface(&config, false, true),
        WindowBackgroundAppearance::Blurred
    );
    assert_eq!(
        appearance_for_surface(&config, true, true),
        WindowBackgroundAppearance::Opaque
    );
}
