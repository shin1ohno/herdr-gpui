use super::{PREVIEW_DELAY, body};
use gpui::TestAppContext;

#[gpui::test]
#[allow(clippy::unwrap_used)]
fn qa_preview_posts_for_the_focused_pane_after_the_delay(cx: &mut TestAppContext) {
    let (view, cx) = cx.add_window_view(crate::sidebar::layout_tests::fixture_window);
    cx.update(|_, cx| super::install_registry(cx));
    cx.update(|window, cx| {
        view.update(cx, |view, cx| window.focus(&view.focus, cx));
        window.draw(cx).clear(cx);
        window.dispatch_action(Box::new(crate::actions::ShowSystemNotificationPreview), cx);
    });
    cx.run_until_parked();
    // Delivery is off in the fixture: the preview ignores it, but waits.
    assert!(cx.shown_system_notifications().is_empty());
    cx.executor().advance_clock(PREVIEW_DELAY);
    cx.run_until_parked();
    let shown = cx.shown_system_notifications();
    let [shown] = shown.as_slice() else {
        panic!("expected one notification: {shown:?}");
    };
    assert_eq!(shown.tag, "herdr-preview:local");
    assert_eq!(shown.title, "Needs attention");
    assert!(shown.body.starts_with("QA preview"));
    view.read_with(cx, |view, _| {
        let (_, notice) = view.endpoints[0].toasts.entries.back().unwrap();
        assert!(notice.posted && !notice.visible);
        assert_eq!(
            notice.pane_id,
            view.live.snapshot.as_ref().unwrap().focused_pane_id
        );
    });
}

#[test]
fn host_label_leads_the_place_line_only_when_given() {
    assert_eq!(body(None, None), "");
    assert_eq!(body(None, Some("Review")), "Review");
    assert_eq!(body(Some("build box"), None), "build box");
    assert_eq!(
        body(Some("build box"), Some("Review")),
        "build box · Review"
    );
    assert_eq!(
        body(Some("build box"), Some("Fix login\nherdr · tests")),
        "Fix login\nbuild box · herdr · tests"
    );
}
