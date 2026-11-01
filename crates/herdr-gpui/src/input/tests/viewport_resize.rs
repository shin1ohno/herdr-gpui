use crate::{
    menu::Page, sidebar::layout_tests::fixture_window, state::ConnectionStatus, window::MockPeer,
};
use gpui::{EntityInputHandler as _, TestAppContext};
use herdr_client::protocol::{ClientMessage, ClientPaneInputEvent};
use std::sync::Arc;

const PANE: &str = "w1:p1";

fn next_input(peer: &mut MockPeer) -> ClientMessage {
    loop {
        let message = peer.receive();
        if matches!(message, ClientMessage::ClientShellPaneInput { .. }) {
            return message;
        }
    }
}

fn typed(text: &str) -> ClientMessage {
    ClientMessage::ClientShellPaneInput {
        pane_id: PANE.into(),
        events: vec![ClientPaneInputEvent::TextCommit(text.into())],
    }
}

#[gpui::test]
fn keyboard_viewport_resize_retains_text_focus_while_input_waits_for_the_frame(
    cx: &mut TestAppContext,
) {
    let mut peer = MockPeer::new();
    let (view, cx) = cx.add_window_view(fixture_window);
    let boot = view.update_in(cx, |view, window, cx| {
        peer.prepare(view, cx);
        window.focus(&view.focus, cx);
        assert!(view.flush_pending_input(cx));
        assert!(view.accepts_text_input(window, cx));

        // A keyboard animation requests several viewport sizes before the
        // daemon's surface arrives. None should resign UIKit's responder.
        let rows = view.options.surface_size.rows;
        for reduction in [4, 6, 8] {
            view.options.surface_size.rows = rows - reduction;
            assert!(!view.input_ready());
            assert!(!view.terminal_accepts_text_input(window));
            assert!(view.accepts_text_input(window, cx));
        }
        view.replace_text_in_range(None, "after resize", window, cx);
        assert_eq!(view.pending_input.len(), 1);
        view.live.snapshot.as_ref().unwrap().boot_id.clone()
    });

    // A directly queued sentinel must arrive before the text retained during
    // the resize: keeping the responder did not send input against stale cells.
    peer.client
        .handle
        .send_input(
            &boot,
            PANE,
            [ClientPaneInputEvent::TextCommit("sentinel".into())],
        )
        .unwrap();
    assert_eq!(next_input(&mut peer), typed("sentinel"));

    view.update_in(cx, |view, window, cx| {
        // Typing notifies the real view, whose layout can also change columns
        // before this update. The daemon's reply must match the whole current
        // viewport, not only the rows changed by the keyboard above.
        let surface = Arc::make_mut(view.live.surface.as_mut().unwrap());
        surface.frame.width = view.options.surface_size.cols;
        surface.frame.height = view.options.surface_size.rows;
        for pane in &mut surface.panes {
            pane.rect.width = surface.frame.width;
            pane.rect.height = surface.frame.height;
            pane.inner_rect.width = surface.frame.width;
            pane.inner_rect.height = surface.frame.height;
        }
        assert!(view.input_ready());
        assert!(view.accepts_text_input(window, cx));
        assert!(view.flush_pending_input(cx));
        assert_eq!(view.pending_input.len(), 0);
    });
    assert_eq!(next_input(&mut peer), typed("after resize"));
}

#[gpui::test]
fn composition_survives_a_keyboard_resize_without_sending(cx: &mut TestAppContext) {
    let peer = MockPeer::new();
    let (view, cx) = cx.add_window_view(fixture_window);
    view.update_in(cx, |view, window, cx| {
        peer.prepare(view, cx);
        window.focus(&view.focus, cx);
        assert!(view.flush_pending_input(cx));
        view.options.surface_size.rows -= 6;
        assert!(!view.input_ready());

        // An input method composing while the keyboard resizes the pane keeps
        // its marked text, which stays local until it is committed.
        view.replace_and_mark_text_in_range(None, "にほ", Some(2..2), window, cx);
        assert_eq!(view.marked_text_range(window, cx), Some(0..2));
        assert_eq!(view.pending_input.len(), 0);

        view.replace_text_in_range(None, "日本", window, cx);
        assert_eq!(view.marked_text_range(window, cx), None);
        assert_eq!(view.pending_input.len(), 1);
    });
}

#[gpui::test]
fn retained_keyboard_focus_ends_with_the_target_connection_or_focus(cx: &mut TestAppContext) {
    let peer = MockPeer::new();
    let (view, cx) = cx.add_window_view(fixture_window);
    view.update_in(cx, |view, window, cx| {
        peer.prepare(view, cx);
        window.focus(&view.focus, cx);
        assert!(view.flush_pending_input(cx));
        view.options.surface_size.rows -= 4;
        view.live.surface = None;
        assert!(!view.input_ready());
        assert!(view.accepts_text_input(window, cx));

        view.menu.page = Some(Page::Menu);
        assert!(!view.accepts_text_input(window, cx));
        view.menu.page = None;
        window.blur(cx);
        assert!(!view.accepts_text_input(window, cx));
        window.focus(&view.focus, cx);
        assert!(view.accepts_text_input(window, cx));

        let boot = view.live.snapshot.as_ref().unwrap().boot_id.clone();
        Arc::make_mut(view.live.snapshot.as_mut().unwrap()).boot_id = "different-boot".into();
        assert!(!view.accepts_text_input(window, cx));
        Arc::make_mut(view.live.snapshot.as_mut().unwrap()).boot_id = boot;
        assert!(view.accepts_text_input(window, cx));

        view.live.status = ConnectionStatus::Disconnected;
        assert!(!view.accepts_text_input(window, cx));
        view.live.status = ConnectionStatus::Connected;
        let handle = view.endpoints[0].connection.handle.take();
        assert!(!view.accepts_text_input(window, cx));
        view.endpoints[0].connection.handle = handle;
        assert!(view.accepts_text_input(window, cx));
        view.clear_pending_input();
        assert!(!view.accepts_text_input(window, cx));
    });
}
