use super::*;

#[gpui::test]
fn a_refused_reservation_does_not_begin_native_acquisition(cx: &mut gpui::TestAppContext) {
    let (view, cx) = cx.add_window_view(crate::sidebar::layout_tests::fixture_window);
    view.update(cx, |view, cx| {
        view.endpoints[view.selected_endpoint].connection.handle = None;
        let began = std::cell::Cell::new(false);
        let delivery = view.start_image_preparation(
            InputTarget::Pane("w1:p1".into()),
            PreparationKind::Input,
            || {
                began.set(true);
                |_: &sheprd_client::ClipboardImageUpload| Ok(Prepared::Empty)
            },
            cx,
        );
        assert!(matches!(delivery, Delivery::Refused));
        assert!(!began.get());
        assert!(view.pending_images.is_empty());
    });
}
