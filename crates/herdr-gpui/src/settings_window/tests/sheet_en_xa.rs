use super::*;
use crate::i18n::leak_tests::{assert_translated, draw, under_en_xa};

#[test]
fn sheet_header_done_and_compact_back_are_translated() {
    let _ipad = crate::ipad_ui::ipad_layout();
    under_en_xa("Settings sheet en-XA", |cx, recorder| {
        let (owner, cx) = cx.add_window_view(crate::sidebar::layout_tests::fixture_window);
        cx.simulate_resize(size(px(320.), px(834.)));
        let fixture = cx.update(|_, cx| sheet::Fixture::prepare(owner.downgrade(), false, cx));
        draw(cx);
        recorder.take();
        cx.update(|_, cx| open_at(owner.downgrade(), None, cx));
        cx.run_until_parked();
        draw(cx);
        assert_translated("Settings sheet navigation/header", &recorder.take(), &[]);
        cx.update(|window, cx| {
            fixture.model.update(cx, |view, cx| {
                view.select_section(Section::General, window, cx)
            })
        });
        draw(cx);
        assert!(cx.debug_bounds("ipad-settings-sheet-back").is_some());
        let data = ["This iPad".to_owned(), "herdr-ipad".to_owned()];
        assert_translated("Settings sheet Back/page", &recorder.take(), &data);
    });
}
