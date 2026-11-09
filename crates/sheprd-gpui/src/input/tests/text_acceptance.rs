use crate::{
    dialog_input::DialogInput, input::TerminalInputHandler, menu::Page,
    sidebar::layout_tests::fixture_window,
};
use gpui::{Bounds, InputHandler as _, TestAppContext};
use sheprd_client::protocol::{ClientShellPopupSurface, FrameData, PaneSurfaceFrame};
use std::sync::Arc;

#[gpui::test]
fn an_empty_or_unfocused_terminal_does_not_request_a_keyboard(cx: &mut TestAppContext) {
    let (view, cx) = cx.add_window_view(fixture_window);
    let mut handler = TerminalInputHandler::new(Bounds::default(), view.clone(), false);
    cx.update(|window, cx| {
        let focus = view.read(cx).focus.clone();
        window.focus(&focus, cx);
        assert!(!handler.accepts_text_input(window, cx));
        assert!(!handler.prefers_ime_for_printable_keys(window, cx));

        view.update(cx, |view, _| view.menu.page = Some(Page::Menu));
        assert!(!handler.accepts_text_input(window, cx));
    });
}

#[gpui::test]
fn a_menu_requests_text_only_while_its_editable_field_has_focus(cx: &mut TestAppContext) {
    let (view, cx) = cx.add_window_view(fixture_window);
    let mut handler = TerminalInputHandler::new(Bounds::default(), view.clone(), true);
    cx.update(|window, cx| {
        view.update(cx, |view, _| {
            view.menu.page = Some(Page::RenameTab);
            view.menu.input = Some(DialogInput::new(String::new()));
        });
        let terminal_focus = view.read(cx).focus.clone();
        window.focus(&terminal_focus, cx);
        assert!(!handler.accepts_text_input(window, cx));

        let menu_focus = view.read(cx).menu.focus.clone();
        window.focus(&menu_focus, cx);
        assert!(handler.accepts_text_input(window, cx));
        assert!(handler.prefers_ime_for_printable_keys(window, cx));

        view.update(cx, |view, _| view.menu.input = None);
        assert!(!handler.accepts_text_input(window, cx));
    });
}

#[gpui::test]
fn a_ready_surface_needs_an_input_target_and_focus_to_request_text(cx: &mut TestAppContext) {
    // The connector refuses in memory: no local socket, process or daemon is
    // used. The view below supplies the coherent projection being tested.
    let client = sheprd_client::connect_with_connector(
        sheprd_client::ConnectTarget::Socket("/unused-input-test".into()),
        sheprd_client::ConnectOptions::default(),
        true,
        |_, _| -> std::io::Result<sheprd_client::Stream> {
            Err(std::io::ErrorKind::NotConnected.into())
        },
    )
    .unwrap();
    let (view, cx) = cx.add_window_view(fixture_window);
    let mut handler = TerminalInputHandler::new(Bounds::default(), view.clone(), false);
    cx.update(|window, cx| {
        view.update(cx, |view, _| {
            let snapshot = view.live.snapshot.as_ref().unwrap();
            view.live.surface = Some(Arc::new(PaneSurfaceFrame {
                boot_id: snapshot.boot_id.clone(),
                projection_revision: snapshot.revision,
                surface_revision: 1,
                frame: FrameData {
                    width: view.options.surface_size.cols,
                    height: view.options.surface_size.rows,
                    cells: vec![],
                    cursor: None,
                    hyperlinks: vec![],
                    graphics: vec![],
                },
                panes: vec![],
                splits: vec![],
                popup: None,
                graphics: Default::default(),
            }));
            view.endpoints[0].attach_shown(client.handle.clone());
            assert!(view.input_ready());
        });
        let focus = view.read(cx).focus.clone();
        window.focus(&focus, cx);
        assert!(!handler.accepts_text_input(window, cx));
        view.update(cx, |view, _| {
            Arc::make_mut(view.live.snapshot.as_mut().unwrap()).focused_pane_id =
                Some("w0:p1".into());
        });
        assert!(handler.accepts_text_input(window, cx));
        assert!(handler.prefers_ime_for_printable_keys(window, cx));

        let other = cx.focus_handle();
        window.focus(&other, cx);
        assert!(!handler.accepts_text_input(window, cx));
        window.focus(&focus, cx);
        view.update(cx, |view, _| {
            Arc::make_mut(view.live.snapshot.as_mut().unwrap()).focused_pane_id = None;
        });
        assert!(!handler.accepts_text_input(window, cx));
        view.update(cx, |view, _| {
            let surface = Arc::make_mut(view.live.surface.as_mut().unwrap());
            surface.popup = Some(Box::new(ClientShellPopupSurface {
                terminal_id: "popup:1".into(),
                title: String::new(),
                width: None,
                height: None,
                frame: surface.frame.clone(),
                mouse_reporting: false,
                sgr_pixel_mouse: false,
                pixel_width: 0,
                pixel_height: 0,
            }));
        });
        assert!(handler.accepts_text_input(window, cx));
        view.update(cx, |view, _| view.live.surface = None);
        assert!(!handler.accepts_text_input(window, cx));
    });
}
