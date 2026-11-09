use super::*;

/// UIKit acquisition must finish on the foreground thread. The worker gets
/// an owned snapshot, so later copies cannot replace a paste already queued.
#[gpui::test]
fn native_image_paste_captures_clipboard_before_worker_runs(cx: &mut gpui::TestAppContext) {
    let (fixture, cx) = cx.add_window_view(|window, cx| {
        Fixture(cx.new(|cx| crate::sidebar::layout_tests::fixture_window(window, cx)))
    });
    let view = fixture.update(cx, |fixture, _| fixture.0.clone());
    let (endpoint, mut server) = connected_endpoint("ssh:captured-image");
    view.update(cx, |view, cx| {
        prepare_remote_image(view, endpoint, cx);
        assert!(view.accepts_clipboard_images());
        cx.write_to_clipboard(clipboard_image(&[42]));
        view.paste_native_clipboard(true, None, cx);
        cx.write_to_clipboard(ClipboardItem::new_string("replacement".into()));
        view.send(ClientPaneInputEvent::TextCommit("sentinel".into()), cx);
    });
    cx.run_until_parked();
    assert_eq!(
        server.receive(),
        ClientMessage::ClipboardImage {
            target: ClientClipboardImageTarget::Pane("w1:p1".into()),
            extension: "png".into(),
            data: vec![42],
        }
    );
    assert_eq!(
        server.receive(),
        ClientMessage::ClientShellPaneInput {
            pane_id: "w1:p1".into(),
            events: vec![ClientPaneInputEvent::TextCommit("sentinel".into())],
        }
    );
    wait_image_finished(&view, cx);
    view.read_with(cx, |view, _| assert!(view.local_error.is_none()));
}
