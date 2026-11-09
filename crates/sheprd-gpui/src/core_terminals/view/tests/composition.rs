use super::super::CoreTerminalView;
use crate::{core_terminals::worker, sidebar::layout_tests::fixture_window};
use gpui::{AppContext, EntityInputHandler, IntoElement, Point, TestAppContext, px, size};
use sheprd_backend::{ClientPaneInputEvent, FrameData, TerminalId};
use std::sync::Arc;
use std::sync::mpsc;

#[gpui::test]
fn composition_paints_until_commit_or_cancel(cx: &mut TestAppContext) {
    let (parent, cx) = cx.add_window_view(fixture_window);
    let (link, commands) = worker::Link::recording();
    let core = parent.update_in(cx, |parent, window, cx| {
        let look = parent.core_look();
        let painter = parent.painter.clone();
        let core = cx.new(|cx| {
            CoreTerminalView::new(
                TerminalId::new("ime").unwrap(),
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
                cells: vec![],
                cursor: Some(sheprd_client::protocol::CursorState {
                    x: 2,
                    y: 1,
                    visible: true,
                    shape: 0,
                }),
                hyperlinks: vec![],
                graphics: vec![],
            }));
            window.focus(&view.focus, cx);
        });
        core
    });
    let paint = |cx: &mut gpui::VisualTestContext| {
        cx.draw(Point::default(), size(px(800.), px(600.)), |_, _| {
            core.clone().into_any_element()
        });
        cx.update(|window, _| window.painted_quads().len())
    };
    let baseline = paint(cx);
    while commands.try_recv().is_ok() {}
    core.update_in(cx, |view, window, cx| {
        view.replace_and_mark_text_in_range(None, "か😀kan", Some(1..3), window, cx);
        assert_eq!(view.marked_text_range(window, cx), Some(0..6));
        let mut adjusted = None;
        assert_eq!(
            view.text_for_range(1..3, &mut adjusted, window, cx),
            Some("😀".into())
        );
        assert_eq!(adjusted, Some(1..3));
        assert_eq!(
            view.selected_text_range(false, window, cx).unwrap().range,
            6..6
        );
    });
    assert!(
        paint(cx) >= baseline + 2,
        "preedit background and underline must be painted"
    );
    assert!(matches!(
        commands.try_recv(),
        Err(mpsc::TryRecvError::Empty)
    ));
    core.update_in(cx, |view, window, cx| {
        view.replace_text_in_range(Some(0..6), "か😀kan", window, cx);
        assert_eq!(view.marked_text_range(window, cx), None);
    });
    assert_eq!(paint(cx), baseline, "commit removes preedit paint");
    assert_eq!(
        commands.try_recv().unwrap(),
        worker::Command::Input(ClientPaneInputEvent::TextCommit("か😀kan".into()))
    );
    assert!(matches!(
        commands.try_recv(),
        Err(mpsc::TryRecvError::Empty)
    ));
    core.update_in(cx, |view, window, cx| {
        view.replace_and_mark_text_in_range(None, "にほんgo", None, window, cx);
    });
    assert!(paint(cx) >= baseline + 2);
    core.update_in(cx, |view, window, cx| view.unmark_text(window, cx));
    assert_eq!(paint(cx), baseline, "cancel removes preedit paint");
    assert!(
        matches!(commands.try_recv(), Err(mpsc::TryRecvError::Empty)),
        "cancel sends no input"
    );
}
