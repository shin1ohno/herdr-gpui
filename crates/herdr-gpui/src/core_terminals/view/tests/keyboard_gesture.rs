use super::super::CoreTerminalView;
use crate::{core_terminals::worker, sidebar::layout_tests::fixture_window};
use gpui::{AppContext as _, EntityInputHandler as _, MouseDownEvent};
use herdr_backend::{Exit, FrameData, TerminalId};
use std::sync::Arc;

#[gpui::test]
fn tapping_a_core_terminal_focuses_only_ready_active_text_input(cx: &mut gpui::TestAppContext) {
    let (parent, cx) = cx.add_window_view(fixture_window);
    parent.update_in(cx, |parent, window, cx| {
        let (link, _commands) = worker::Link::recording();
        let look = parent.core_look();
        let painter = parent.painter.clone();
        let core = cx.new(|cx| {
            CoreTerminalView::new(
                TerminalId::new("t1").unwrap(),
                link,
                look,
                painter,
                false,
                cx,
            )
        });
        core.update(cx, |view, cx| {
            view.screen.frame = Some(Arc::new(FrameData {
                width: 80,
                height: 24,
                cells: Vec::new(),
                cursor: None,
                hyperlinks: Vec::new(),
                graphics: Vec::new(),
            }));
            window.focus(&parent.focus, cx);
            assert!(!view.accepts_text_input(window, cx));
            view.mouse_down(&MouseDownEvent::default(), window, cx);
            assert!(view.focus.is_focused(window));
            assert!(view.accepts_text_input(window, cx));

            view.observe = true;
            view.mouse_down(&MouseDownEvent::default(), window, cx);
            assert!(!view.accepts_text_input(window, cx));
            view.observe = false;
            view.screen.ended = Some(Exit { code: Some(0) });
            view.mouse_down(&MouseDownEvent::default(), window, cx);
            assert!(!view.accepts_text_input(window, cx));
            view.screen.ended = None;
            view.screen.failure = Some("connection ended".into());
            view.mouse_down(&MouseDownEvent::default(), window, cx);
            assert!(!view.accepts_text_input(window, cx));
            view.screen.failure = None;
            view.screen.frame = None;
            view.mouse_down(&MouseDownEvent::default(), window, cx);
            assert!(!view.accepts_text_input(window, cx));
        });
    });
}
