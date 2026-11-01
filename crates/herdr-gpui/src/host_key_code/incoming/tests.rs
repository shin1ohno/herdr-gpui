use super::*;

#[test]
fn a_url_goes_only_to_the_active_main_window() {
    let first = WindowId::from(1);
    let second = WindowId::from(2);
    assert_eq!(recipient(Some(second), [first, second]), Some(second));
    assert_eq!(recipient(Some(first), [first, second]), Some(first));
}

#[test]
fn an_ambiguous_scene_is_refused_and_a_sole_main_window_can_receive() {
    let first = WindowId::from(1);
    let second = WindowId::from(2);
    let settings = WindowId::from(3);
    assert_eq!(recipient(None, [first, second]), None);
    assert_eq!(recipient(Some(settings), [first, second]), None);
    assert_eq!(recipient(None, [first]), Some(first));
    assert_eq!(recipient(Some(settings), [first]), Some(first));
    assert_eq!(recipient(None, []), None);
}

#[gpui::test]
fn urls_without_a_main_window_are_inert(cx: &mut gpui::TestAppContext) {
    cx.update(|cx| {
        open_urls(vec!["https://example.test".into()], cx);
        open_urls(vec!["herdr-ipad://host-key?v=1".into()], cx);
        assert!(cx.windows().is_empty());
    });
}
