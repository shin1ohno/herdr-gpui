use super::*;
use crate::{menu::Page, state::ConnectionStatus};

fn draw(cx: &mut gpui::VisualTestContext) {
    cx.update(|window, cx| full_draw(window, cx).clear(cx));
}

fn click(cx: &mut gpui::VisualTestContext, key: &'static str) {
    let point = cx.debug_bounds(key).unwrap().center();
    cx.simulate_click(point, Modifiers::default());
    cx.run_until_parked();
    draw(cx);
}

#[gpui::test]
fn touch_search_popup_owns_input_closes_and_reopens_fresh(cx: &mut gpui::TestAppContext) {
    let _ipad = crate::ipad_ui::ipad_layout();
    let (view, cx) = cx.add_window_view(connected);
    cx.simulate_resize(size(px(800.), px(700.)));
    cx.update(|window, cx| window.focus(&view.read(cx).focus.clone(), cx));
    draw(cx);
    assert!(cx.debug_bounds("sidebar-search").is_none());
    click(cx, "sidebar-search-open");
    assert!(cx.debug_bounds("sidebar-search-popup").is_some());
    view.read_with(cx, |view, _| {
        assert_eq!(view.menu.page, Some(Page::SidebarSearch))
    });
    cx.simulate_input("sidebar-child");
    cx.run_until_parked();
    draw(cx);
    assert!(cx.debug_bounds("sidebar-search-hit-0").is_some());
    // A popup query never replaces the underlying iPad spaces list.
    assert!(cx.debug_bounds("spaces-scroll").is_some());
    cx.simulate_keystrokes("escape");
    cx.update(|window, cx| {
        assert!(view.read(cx).menu.page.is_none());
        assert!(view.read(cx).focus.is_focused(window));
    });
    click(cx, "sidebar-search-open");
    view.read_with(cx, |view, _| assert!(view.sidebar_search.query().is_none()));
    cx.simulate_input("no such workspace");
    draw(cx);
    assert!(cx.debug_bounds("sidebar-search-hit-0").is_none());
    click(cx, "sidebar-search-close");
    view.read_with(cx, |view, _| assert!(view.menu.page.is_none()));
    click(cx, "sidebar-search-open");
    cx.simulate_click(point(px(780.), px(680.)), Modifiers::default());
    draw(cx);
    view.read_with(cx, |view, _| assert!(view.menu.page.is_none()));
}

#[gpui::test]
fn popup_results_use_host_identity_and_refuse_disconnected_spaces(cx: &mut gpui::TestAppContext) {
    let _ipad = crate::ipad_ui::ipad_layout();
    let (view, cx) = cx.add_window_view(connected);
    view.update(cx, |view, _| {
        let mut remote = crate::endpoint::Endpoint::new(
            "ssh:search".into(),
            "Remote search host".into(),
            ConnectTarget::Ssh {
                target: "unused".into(),
                session: "default".into(),
            },
            true,
        );
        remote.live.snapshot = Some(Arc::new(snapshot(6)));
        remote.live.status = ConnectionStatus::Detached;
        view.endpoints.push(remote);
    });
    cx.update(|window, cx| {
        view.update(cx, |view, cx| {
            view.open_sidebar_search_popup(point(px(100.), px(100.)), window, cx);
            let target = crate::sidebar::search::Target::Workspace {
                endpoint: "ssh:search".into(),
                workspace: "w5".into(),
            };
            view.open_search_hit(&target, window, cx);
            assert_eq!(view.menu.page, Some(Page::SidebarSearch));
            assert!(view.menu.error.is_some());
            assert_eq!(view.selected_endpoint, 0);
            view.endpoints[1].live.status = ConnectionStatus::Connected;
            // Endpoint switching reads the stored source endpoint, as the inbox
            // normally synchronizes it with the selected live projection.
            view.live.supports_surface = false;
            view.endpoints[0].live.supports_surface = false;
            view.open_search_hit(&target, window, cx);
            assert_eq!(view.menu.page, Some(Page::SidebarSearch));
            assert!(view.menu.error.is_some());
            assert_eq!(view.selected_endpoint, 0);
            view.live.supports_surface = true;
            view.endpoints[0].live.supports_surface = true;
            view.open_search_hit(&target, window, cx);
            assert!(view.menu.page.is_none());
            assert_eq!(view.selected_endpoint, 1);
            assert_eq!(
                view.pending_navigation,
                Some(crate::NavigationTarget::Workspace("w5".into()))
            );
        })
    });
}

#[gpui::test]
fn header_and_rail_controls_have_separate_touch_targets_in_both_directions(
    cx: &mut gpui::TestAppContext,
) {
    let _ipad = crate::ipad_ui::ipad_layout();
    for locale in [
        sheprd_i18n::Locale::FALLBACK,
        sheprd_i18n::Pseudo::Bidi.locale(),
    ] {
        crate::i18n::show(&locale);
        let (view, cx) = cx.add_window_view(connected);
        cx.simulate_resize(size(px(800.), px(700.)));
        view.update(cx, |view, _| view.sidebar_width = Some(160.));
        draw(cx);
        let search = cx.debug_bounds("sidebar-search-open").unwrap();
        let new_space = cx.debug_bounds("sidebar-new-space").unwrap();
        for bounds in [search, new_space] {
            assert!(bounds.size.width >= px(44.) && bounds.size.height >= px(44.));
        }
        assert!(search.right() <= new_space.left() || new_space.right() <= search.left());
        assert!(cx.debug_bounds("new-workspace").is_none());
        view.update(cx, |view, cx| {
            view.sidebar_visible = false;
            cx.notify();
        });
        draw(cx);
        let search = cx.debug_bounds("rail-search").unwrap();
        let new_space = cx.debug_bounds("rail-new-workspace").unwrap();
        assert!(search.size.width >= px(44.) && new_space.size.width >= px(44.));
        assert!(search.bottom() <= new_space.top());
        for key in ["rail-devices", "rail-settings", "rail-agents"] {
            assert!(cx.debug_bounds(key).is_some(), "{key}");
        }
        // Restore the expanded preference before probing its compact overlay;
        // a collapsed preference deliberately keeps the rail inside it.
        view.update(cx, |view, cx| {
            view.sidebar_visible = true;
            cx.notify();
        });
        cx.simulate_resize(size(px(545.), px(700.)));
        draw(cx);
        assert!(cx.debug_bounds("sidebar-search-open").is_none());
        click(cx, "ipad-compact-menu");
        assert!(view.read_with(cx, |view, _| view.ipad.overlay_open.is_open()));
        assert!(cx.debug_bounds("ipad-compact-sidebar").is_some());
        let search = cx.debug_bounds("sidebar-search-open").unwrap();
        let new_space = cx.debug_bounds("sidebar-new-space").unwrap();
        for bounds in [search, new_space] {
            assert!(bounds.size.width >= px(44.) && bounds.size.height >= px(44.));
        }
        assert!(search.right() <= new_space.left() || new_space.right() <= search.left());
        click(cx, "sidebar-search-open");
        assert!(cx.debug_bounds("sidebar-search-popup").is_some());
        cx.simulate_input("sidebar-child");
        cx.run_until_parked();
        draw(cx);
        assert!(cx.debug_bounds("sidebar-search-hit-0").is_some());
    }
    crate::i18n::show(&sheprd_i18n::Locale::FALLBACK);
}

#[gpui::test]
fn popup_arrows_and_enter_select_results(cx: &mut gpui::TestAppContext) {
    let _ipad = crate::ipad_ui::ipad_layout();
    let (view, cx) = cx.add_window_view(connected);
    cx.update(|window, cx| {
        view.update(cx, |view, cx| {
            view.open_sidebar_search_popup(point(px(100.), px(100.)), window, cx);
        })
    });
    cx.simulate_input("sidebar-child");
    cx.run_until_parked();
    draw(cx);
    cx.simulate_keystrokes("down enter");
    cx.update(|window, cx| {
        let view = view.read(cx);
        assert!(view.menu.page.is_none());
        assert_eq!(
            view.pending_navigation,
            Some(crate::NavigationTarget::Workspace("w5".into()))
        );
        assert!(view.focus.is_focused(window));
    });
}

#[gpui::test]
fn popup_input_never_reaches_the_terminal_peer(cx: &mut gpui::TestAppContext) {
    use sheprd_client::protocol::{ClientMessage, ClientPaneInputEvent};

    let _ipad = crate::ipad_ui::ipad_layout();
    let mut peer = crate::window::MockPeer::new();
    let (view, cx) = cx.add_window_view(fixture_window);
    cx.update(|window, cx| {
        view.update(cx, |view, cx| {
            peer.prepare(view, cx);
            window.focus(&view.focus, cx);
            assert!(view.flush_pending_input(cx));
            view.open_sidebar_search_popup(point(px(100.), px(100.)), window, cx);
        });
    });
    cx.simulate_input("no-matching-popup-result");
    cx.run_until_parked();
    draw(cx);
    cx.simulate_keystrokes("down enter escape");
    cx.update(|window, cx| {
        view.update(cx, |view, cx| {
            assert!(view.menu.page.is_none());
            assert!(view.focus.is_focused(window));
            // Drawing asks the mock daemon for the window's actual surface size.
            // Acknowledge that resize before flushing, as a real peer would.
            let requested = view.options.surface_size;
            let surface = Arc::make_mut(view.live.surface.as_mut().unwrap());
            surface.frame.width = requested.cols;
            surface.frame.height = requested.rows;
            assert!(view.flush_pending_input(cx));
        });
    });
    // The ordered transport sentinel exposes any text or key leaked by the popup.
    peer.client
        .handle
        .send_input(
            "boot-v1",
            "w1:p1",
            [ClientPaneInputEvent::Paste("sentinel".into())],
        )
        .unwrap();
    loop {
        let message = peer.receive();
        if matches!(
            message,
            ClientMessage::ClientShellPaneInput { .. }
                | ClientMessage::ClientShellPopupInput { .. }
        ) {
            assert_eq!(
                message,
                ClientMessage::ClientShellPaneInput {
                    pane_id: "w1:p1".into(),
                    events: vec![ClientPaneInputEvent::Paste("sentinel".into())],
                }
            );
            break;
        }
    }
}
