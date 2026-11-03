use crate::{
    HerdrWindow,
    i18n::leak_tests::{assert_translated, draw, uncut, under_en_xa},
    sidebar::layout_tests::fixture_window,
};
use core::prelude::v1::test;
use gpui::{prelude::*, *};

struct Keys(Entity<HerdrWindow>);

impl Render for Keys {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.0.update(cx, |view, cx| {
            view.viewport_width = f32::from(window.viewport_size().width);
            div()
                .size_full()
                .flex()
                .flex_col()
                .children(view.ipad_compact_header(window, cx))
                .children(view.ipad_layers(window, cx))
        })
    }
}

#[test]
fn keyboard_band_translates_every_label() {
    localized_keys("iPad key band", 1194., 300., "ipad-key-band");
}

#[test]
fn expanded_compact_keys_translate_every_label() {
    localized_keys("iPad compact keys", 320., 0., "ipad-compact-key-row");
}

fn localized_keys(name: &'static str, width: f32, inset: f32, selector: &'static str) {
    under_en_xa(name, |cx, recorder| {
        let _ipad = crate::ipad_ui::ipad_layout();
        let (surface, cx) =
            cx.add_window_view(|window, cx| Keys(cx.new(|cx| fixture_window(window, cx))));
        cx.simulate_resize(size(px(width), px(834.)));
        cx.simulate_window_visual_viewport_change(
            cx.window_handle(),
            Bounds::new(point(px(0.), px(0.)), size(px(width), px(834. - inset))),
        );
        let view = surface.read_with(cx, |surface, _| surface.0.clone());
        view.update(cx, |view, cx| {
            view.ipad.more_keys.set(true, crate::motion::now(), true);
            cx.notify();
        });
        draw(cx);
        assert!(cx.debug_bounds(selector).is_some());
        assert_translated(name, &uncut(recorder.take()), &[]);
    });
}
