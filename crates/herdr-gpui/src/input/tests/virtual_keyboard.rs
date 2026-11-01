use crate::{
    HerdrWindow, dialog_input::DialogInput, menu::Page, search_input::SearchInput,
    sidebar::layout_tests::fixture_window,
};
use gpui::{
    AppContext as _, Context, Entity, EntityInputHandler as _, Modifiers, Render, TestAppContext,
    Window, prelude::*,
};
use herdr_client::protocol::{
    ClientShellPopupSurface, FrameData, PaneSurfaceFrame, PaneSurfacePane, SurfaceRect,
};
use std::sync::Arc;

#[gpui::test]
fn terminal_keyboard_eligibility_excludes_copy_mode_menus_and_unready_targets(
    cx: &mut TestAppContext,
) {
    let client = herdr_client::connect_with_connector(
        herdr_client::ConnectTarget::Socket("/unused-keyboard-test".into()),
        herdr_client::ConnectOptions::default(),
        true,
        |_, _| -> std::io::Result<herdr_client::Stream> {
            Err(std::io::ErrorKind::NotConnected.into())
        },
    )
    .unwrap();
    let (view, cx) = cx.add_window_view(fixture_window);
    view.update_in(cx, |view, window, cx| {
        window.focus(&view.focus, cx);
        assert!(!view.terminal_accepts_text_input(window));
        let snapshot = Arc::make_mut(view.live.snapshot.as_mut().unwrap());
        snapshot.focused_pane_id = Some("w0:p1".into());
        let rect = SurfaceRect {
            x: 0,
            y: 0,
            width: view.options.surface_size.cols,
            height: view.options.surface_size.rows,
        };
        view.live.surface = Some(Arc::new(PaneSurfaceFrame {
            boot_id: snapshot.boot_id.clone(),
            projection_revision: snapshot.revision,
            surface_revision: 1,
            frame: FrameData {
                width: rect.width,
                height: rect.height,
                cells: vec![],
                cursor: None,
                hyperlinks: vec![],
                graphics: vec![],
            },
            panes: vec![PaneSurfacePane {
                pane_id: "w0:p1".into(),
                content_revision: 1,
                rect,
                inner_rect: rect,
                scrollbar_rect: None,
                scroll: None,
                focused: true,
                mouse_reporting: false,
                sgr_pixel_mouse: false,
                alternate_screen_active: false,
                pixel_width: 0,
                pixel_height: 0,
            }],
            splits: vec![],
            popup: None,
            graphics: Default::default(),
        }));
        view.endpoints[0].attach_shown(client.handle.clone());
        assert!(view.terminal_accepts_text_input(window));

        view.options.surface_size.cols += 1;
        assert!(!view.terminal_accepts_text_input(window));
        view.options.surface_size.cols -= 1;
        view.menu.page = Some(Page::Menu);
        assert!(!view.terminal_accepts_text_input(window));
        view.menu.page = None;

        view.live.supports_copy_motion = true;
        view.enter_copy_mode(window, cx);
        assert!(view.copy_mode_active());
        // Copy mode still accepts navigation keys through its input handler,
        // but a pane gesture must not summon the software keyboard for it.
        assert!(view.accepts_text_input(window, cx));
        assert!(!view.terminal_accepts_text_input(window));
        view.leave_copy_mode(cx);
        assert!(view.terminal_accepts_text_input(window));

        Arc::make_mut(view.live.snapshot.as_mut().unwrap()).focused_pane_id = None;
        assert!(!view.terminal_accepts_text_input(window));
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
        assert!(view.terminal_accepts_text_input(window));
        window.blur(cx);
        assert!(!view.terminal_accepts_text_input(window));
    });
}

#[gpui::test]
fn a_search_field_can_take_text_again_after_keyboard_dismissal(cx: &mut TestAppContext) {
    let (input, cx) = cx.add_window_view(|_, cx| SearchInput::new(cx));
    cx.update(|window, cx| window.draw(cx).clear(cx));
    let field = cx.debug_bounds("theme-search").unwrap();
    for text in ["a", "b"] {
        cx.update(|window, cx| {
            window.dismiss_virtual_keyboard();
            window.blur(cx);
        });
        cx.simulate_click(field.center(), Modifiers::default());
        input.update_in(cx, |input, window, cx| {
            assert!(input.focus.is_focused(window));
            input.replace_text_in_range(None, text, window, cx);
            assert!(input.text().contains(text));
        });
    }
}

struct DialogField(Entity<HerdrWindow>);

impl Render for DialogField {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.0.update(cx, |view, cx| {
            view.render_dialog_input(cx).track_focus(&view.menu.focus)
        })
    }
}

#[gpui::test]
fn a_dialog_field_can_take_focus_back_after_keyboard_dismissal(cx: &mut TestAppContext) {
    let (host, cx) = cx.add_window_view(|window, cx| {
        DialogField(cx.new(|cx| {
            let mut view = fixture_window(window, cx);
            view.menu.page = Some(Page::RenameTab);
            view.menu.input = Some(DialogInput::new(String::new()));
            view
        }))
    });
    let view = host.read_with(cx, |host, _| host.0.clone());
    cx.update(|window, cx| window.draw(cx).clear(cx));
    let field = cx.debug_bounds("dialog-input").unwrap();
    for text in ["a", "b"] {
        cx.update(|window, cx| {
            window.dismiss_virtual_keyboard();
            window.blur(cx);
        });
        cx.simulate_click(field.center(), Modifiers::default());
        view.update_in(cx, |view, window, cx| {
            assert!(view.menu.focus.is_focused(window));
            assert!(view.accepts_text_input(window, cx));
            view.replace_text_in_range(None, text, window, cx);
            assert!(view.menu.input.as_ref().unwrap().text.contains(text));
        });
    }
}
