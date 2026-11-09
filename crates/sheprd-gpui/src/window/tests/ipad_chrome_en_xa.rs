use crate::{
    SheprdWindow,
    config::ShortcutStyle,
    controls::Command,
    i18n::leak_tests::{assert_translated, draw, uncut, under_en_xa},
    sidebar::layout_tests::fixture_window,
};
use core::prelude::v1::test;
use gpui::{prelude::*, *};

struct Chrome(Entity<SheprdWindow>);

impl Render for Chrome {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.0.update(cx, |view, cx| view.render_status_bar(cx))
    }
}

#[test]
fn ipad_chrome_translates_shortcut_button_and_hardware_prose() {
    under_en_xa("iPad chrome", |cx, recorder| {
        let _ipad = crate::ipad_ui::ipad_layout();
        let (surface, cx) = cx.add_window_view(|window, cx| {
            let view = cx.new(|cx| {
                let mut view = fixture_window(window, cx);
                view.config.shortcut_style = ShortcutStyle::Ipad;
                view
            });
            cx.observe(&view, |_, _, cx| cx.notify()).detach();
            Chrome(view)
        });
        cx.simulate_resize(size(px(1800.), px(834.)));
        draw(cx);
        let mut seen = recorder.take();
        let view = surface.read_with(cx, |surface, _| surface.0.clone());
        let mut data = Vec::new();
        for style in [ShortcutStyle::Ipad, ShortcutStyle::HerdrPrefix] {
            view.update(cx, |view, cx| {
                view.config.shortcut_style = style;
                for command in [
                    Command::Tab,
                    Command::SplitRight,
                    Command::ToggleSidebar,
                    Command::Palette,
                    Command::ClosePane,
                    Command::Keybinds,
                ] {
                    if let Some(hint) = view.shortcut_hint(command) {
                        data.push(hint);
                    }
                }
                cx.notify();
            });
            cx.simulate_keystrokes("escape");
            draw(cx);
            seen.extend(recorder.take());
            assert!(cx.debug_bounds("ipad-status-hardware").is_some());
            assert!(cx.debug_bounds("status-keybinds").is_some());
            assert_translated("iPad status chrome", &uncut(seen.clone()), &data);
        }
    });
}
