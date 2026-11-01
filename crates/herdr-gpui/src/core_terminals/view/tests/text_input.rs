use super::super::CoreTerminalView;
use crate::{core_terminals::worker, sidebar::layout_tests::fixture_window};
use gpui::{AppContext, Context, EntityInputHandler, Window};
use herdr_backend::{ClientPaneInputEvent, Exit, FrameData, TerminalId};
use std::{sync::Arc, sync::mpsc};

fn rejects_text(
    view: &mut CoreTerminalView,
    window: &mut Window,
    cx: &mut Context<CoreTerminalView>,
    commands: &mpsc::Receiver<worker::Command>,
) {
    assert!(!view.accepts_text_input(window, cx));
    view.replace_text_in_range(None, "must not type", window, cx);
    view.replace_and_mark_text_in_range(None, "must not compose", None, window, cx);
    assert!(view.marked.is_empty());
    assert!(matches!(
        commands.try_recv(),
        Err(mpsc::TryRecvError::Empty)
    ));
}

#[gpui::test]
fn only_a_focused_ready_active_terminal_accepts_text(cx: &mut gpui::TestAppContext) {
    let (parent, cx) = cx.add_window_view(fixture_window);
    parent.update_in(cx, |parent, window, cx| {
        let (link, commands) = worker::Link::recording();
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
            window.focus(&view.focus, cx);
            rejects_text(view, window, cx, &commands);
            view.screen.frame = Some(Arc::new(FrameData {
                width: 80,
                height: 24,
                cells: Vec::new(),
                cursor: None,
                hyperlinks: Vec::new(),
                graphics: Vec::new(),
            }));
            assert!(view.accepts_text_input(window, cx));
            view.replace_text_in_range(None, "types", window, cx);
            assert_eq!(
                commands.try_recv().unwrap(),
                worker::Command::Input(ClientPaneInputEvent::TextCommit("types".into()))
            );
            view.replace_and_mark_text_in_range(None, "composes", None, window, cx);
            assert_eq!(view.marked, "composes");
            view.unmark_text(window, cx);

            view.observe = true;
            rejects_text(view, window, cx, &commands);
            view.observe = false;
            view.screen.ended = Some(Exit { code: Some(0) });
            rejects_text(view, window, cx, &commands);
            view.screen.ended = None;
            view.screen.failure = Some("connection ended".into());
            rejects_text(view, window, cx, &commands);
            view.screen.failure = None;
            window.focus(&parent.focus, cx);
            rejects_text(view, window, cx, &commands);
        });
    });
}
