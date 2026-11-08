use super::*;
use crate::{
    config::ShortcutStyle,
    i18n::leak_tests::{draw, under_en_xa},
    keymap::{DaemonKeys, Keymap},
};
use core::prelude::v1::test;
use gpui::{prelude::*, *};
use sheprd_i18n::{Language, Locale};

fn english(cx: &mut TestAppContext) {
    cx.update(|cx| crate::i18n::install(&Language::Locale(Locale::FALLBACK), cx));
}

fn screen(window: &mut Window, cx: &mut Context<SheprdWindow>) -> SheprdWindow {
    let mut view = fixture_window(window, cx);
    view.config.shortcut_style = ShortcutStyle::HerdrPrefix;
    let mut listing = snapshot(1);
    listing.focused_workspace_id = Some("w0".into());
    listing.focused_tab_id = Some("t0".into());
    view.live.snapshot = Some(Arc::new(listing));
    window.focus(&view.focus, cx);
    view
}

#[test]
fn screen_08_renders_the_herdr_tab_chip_guide_and_sidebar_hint() {
    under_en_xa("screen 08 Herdr prefix", |cx, recorder| {
        let _ipad = crate::ipad_ui::ipad_layout();
        english(cx);
        let (view, cx) = cx.add_window_view(screen);
        cx.simulate_resize(size(px(1600.), px(834.)));
        draw(cx);
        cx.simulate_keystrokes("escape");
        draw(cx);
        assert!(cx.debug_bounds("tab-shortcut-t0").is_some());
        let seen = recorder.take();
        assert!(seen.iter().any(|line| line == "⌃B 1"), "{seen:?}");
        assert!(seen.iter().any(|line| line == "Shortcuts"), "{seen:?}");
        assert!(cx.debug_bounds("status-keybinds").is_some());
        view.update_in(cx, |view, window, cx| {
            view.command(Command::Keybinds, window, cx)
        });
        draw(cx);
        assert!(cx.debug_bounds("shortcut-guide").is_some());
        let guide = recorder.take();
        for key in ["⌃B", "w", "b", "s"] {
            assert!(guide.iter().any(|line| line == key), "{guide:?}");
        }
        view.update(cx, |view, cx| {
            view.disarm_prefix();
            view.sidebar_visible = false;
            cx.notify();
        });
        draw(cx);
        assert!(cx.debug_bounds("ipad-rail-hint-shortcut").is_some());
        assert!(recorder.take().iter().any(|line| line == "⌃B b"));
        assert_eq!(
            view.read_with(cx, |view, _| view.shortcut_hint(Command::ToggleSidebar))
                .as_deref(),
            Some("⌃B b")
        );
    });
}

struct PaletteOnly(Entity<SheprdWindow>);

impl Render for PaletteOnly {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div().size_full().children(self.0.update(cx, |view, cx| {
            view.menu.page.map(|_| view.render_menu(window, cx))
        }))
    }
}

#[test]
fn screen_08_palette_renders_resolved_herdr_chips() {
    under_en_xa("screen 08 palette", |cx, recorder| {
        let _ipad = crate::ipad_ui::ipad_layout();
        english(cx);
        let (surface, cx) = cx.add_window_view(|window, cx| {
            let view = cx.new(|cx| screen(window, cx));
            cx.observe(&view, |_, _, cx| cx.notify()).detach();
            PaletteOnly(view)
        });
        cx.simulate_resize(size(px(1194.), px(834.)));
        draw(cx);
        cx.simulate_keystrokes("escape");
        let view = surface.read_with(cx, |surface, _| surface.0.clone());
        for (prefix, hint) in [("ctrl+b", "⌃B c"), ("ctrl+a", "⌃A c")] {
            view.update_in(cx, |view, window, cx| {
                let table = format!("prefix = '{prefix}'").parse().unwrap();
                view.config.keybindings = Keymap::with_overrides(
                    &Default::default(),
                    &Default::default(),
                    &DaemonKeys::from_table(Some(&table)),
                )
                .unwrap();
                view.open_palette(crate::palette::Filter::Commands, window, cx);
                view.filter_palette("New tab", cx);
            });
            cx.run_until_parked();
            draw(cx);
            assert!(cx.debug_bounds("palette-row-0").is_some());
            assert!(cx.debug_bounds("ipad-kbd").is_some());
            assert!(recorder.take().iter().any(|line| line == hint));
            view.update_in(cx, |view, window, cx| view.dismiss_menu(window, cx));
            draw(cx);
        }
    });
}

#[gpui::test]
fn screen_08_hints_follow_a_resolved_nondefault_prefix(cx: &mut TestAppContext) {
    let _ipad = crate::ipad_ui::ipad_layout();
    let (view, cx) = cx.add_window_view(screen);
    view.update(cx, |view, _| {
        let table = "prefix = 'ctrl+a'".parse().unwrap();
        view.config.keybindings = Keymap::with_overrides(
            &Default::default(),
            &Default::default(),
            &DaemonKeys::from_table(Some(&table)),
        )
        .unwrap();
        for (command, hint) in [
            (Command::TabNumber(1), "⌃A 1"),
            (Command::Tab, "⌃A c"),
            (Command::SplitRight, "⌃A v"),
            (Command::ClosePane, "⌃A x"),
            (Command::Keybinds, "⌃A ?"),
            (Command::ToggleSidebar, "⌃A b"),
        ] {
            assert_eq!(view.shortcut_hint(command).as_deref(), Some(hint));
        }
    });
}
