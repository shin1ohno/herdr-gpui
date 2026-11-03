use super::*;

#[test]
fn catalog_bindings_are_unique_parseable_and_keep_text_separate() {
    for (index, key) in Key::ALL.iter().enumerate() {
        assert!(!Key::ALL[..index].contains(key));
        assert!(key.width() >= 44.);
        if let Binding::Keystroke(source) = key.binding() {
            assert!(Keystroke::parse(source).is_ok(), "{source}");
        }
    }
    assert_eq!(Key::Escape.binding(), Binding::Keystroke("escape"));
    assert_eq!(Key::Tab.binding(), Binding::Keystroke("tab"));
    assert_eq!(Key::Pipe.binding(), Binding::Text("|"));
    assert_eq!(Key::Tilde.binding(), Binding::Text("~"));
    assert_eq!(Key::Slash.binding(), Binding::Text("/"));
    assert_eq!(Key::Dash.binding(), Binding::Text("-"));
    assert_eq!(Key::Paste.binding(), Binding::Paste);
    assert_eq!(Key::Hide.binding(), Binding::Hide);
}

#[test]
fn paging_wraps_and_refuses_stale_dot_indices() {
    assert_eq!(Page::Next.index(2, 3), Some(0));
    assert_eq!(Page::Previous.index(0, 3), Some(2));
    assert_eq!(Page::At(1).index(0, 3), Some(1));
    assert_eq!(Page::At(3).index(0, 3), None);
    assert_eq!(Page::Next.index(0, 0), None);
    assert_eq!(Page::Previous.index(3, 3), None);
}

#[test]
fn pager_swipes_complete_once_and_taps_select_their_dot() {
    use gpui::{TouchDragEvent, TouchPhase, point, px};
    let event = |phase, x, y| TouchDragEvent {
        phase,
        start_position: point(px(0.), px(0.)),
        position: point(px(x), px(y)),
    };
    for phase in [
        TouchPhase::Started,
        TouchPhase::Moved,
        TouchPhase::Cancelled,
    ] {
        assert_eq!(band::drag_page(&event(phase, -80., 0.), None), None);
    }
    assert_eq!(
        band::drag_page(&event(TouchPhase::Ended, -80., 5.), None),
        Some(Page::Next)
    );
    assert_eq!(
        band::drag_page(&event(TouchPhase::Ended, 80., 5.), None),
        Some(Page::Previous)
    );
    assert_eq!(
        band::drag_page(&event(TouchPhase::Ended, 35., 80.), None),
        None
    );
    assert_eq!(
        band::drag_page(&event(TouchPhase::Ended, 3., 2.), Some(Page::At(1))),
        Some(Page::At(1))
    );
}
