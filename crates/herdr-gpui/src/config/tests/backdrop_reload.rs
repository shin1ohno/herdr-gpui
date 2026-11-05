#![allow(clippy::unwrap_used)]

use super::*;
use crate::{config::backdrop::Edit, sidebar::layout_tests::fixture_window};

#[gpui::test]
fn backdrop_reload_uses_the_latest_saved_value_and_keeps_it_on_a_failed_load(
    cx: &mut gpui::TestAppContext,
) {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("config-gpui.local.toml");
    let (view, cx) = cx.add_window_view(fixture_window);
    for (opacity, blur) in [(0.2, true), (0.8, false), (1., true)] {
        Config::save_backdrop_path(&Edit::Opacity(opacity), &path).unwrap();
        Config::save_backdrop_path(&Edit::Blur(blur), &path).unwrap();
        let saved = path.clone();
        view.update(cx, |view, cx| {
            view.load_gui_config_with(
                move || {
                    let text = fs::read_to_string(&saved)?;
                    let config = Config::parse(&text)?;
                    let theme = config.theme(false)?;
                    Ok((config, theme))
                },
                cx,
            );
        });
        cx.run_until_parked();
        view.read_with(cx, |view, _| {
            assert_eq!(view.config.backdrop.opacity, opacity);
            assert_eq!(view.config.backdrop.blur, blur);
            let effective = if cfg!(target_os = "ios") { 1. } else { opacity };
            assert_eq!(view.fill().ground().a, effective);
            // A fresh parse, as at restart, retains the same portable settings.
            let restarted = Config::parse(&fs::read_to_string(&path).unwrap()).unwrap();
            assert_eq!(restarted.backdrop, view.config.backdrop);
        });
    }
    view.update(cx, |view, cx| {
        view.load_gui_config_with(|| Err(crate::Error::MissingHome), cx);
    });
    cx.run_until_parked();
    view.read_with(cx, |view, _| {
        assert_eq!(view.config.backdrop.opacity, 1.);
        assert!(view.config.backdrop.blur);
        assert!(view.local_error.is_some());
    });
}
