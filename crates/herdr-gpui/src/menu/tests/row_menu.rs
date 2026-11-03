#![allow(clippy::unwrap_used)]
use crate::{
    HerdrWindow, NavigationTarget,
    menu::Page,
    sidebar::layout_tests::{fixture_window, full_draw, snapshot},
};
use core::prelude::v1::test;
use gpui::{prelude::*, *};
use std::sync::Arc;

struct MenuSurface(Entity<HerdrWindow>);
impl Render for MenuSurface {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div().size_full().children(self.0.update(cx, |view, cx| {
            view.menu.page.map(|_| view.render_menu(window, cx))
        }))
    }
}
fn fixture(cx: &mut TestAppContext) -> (Entity<HerdrWindow>, &mut VisualTestContext) {
    let (surface, cx) = cx.add_window_view(|window, cx| {
        let view = cx.new(|cx| {
            let mut view = fixture_window(window, cx);
            view.live.snapshot = Some(Arc::new(snapshot(2)));
            view.sidebar_visible = false;
            view
        });
        cx.observe(&view, |_, _, cx| cx.notify()).detach();
        MenuSurface(view)
    });
    let view = surface.read_with(cx, |surface, _| surface.0.clone());
    cx.simulate_resize(size(px(1194.), px(834.)));
    (view, cx)
}
fn draw(cx: &mut VisualTestContext) {
    cx.update(|window, cx| full_draw(window, cx).clear(cx));
}
fn open(
    view: &Entity<HerdrWindow>,
    cx: &mut VisualTestContext,
    target: Option<NavigationTarget<&str>>,
) {
    cx.update(|window, cx| {
        view.update(cx, |view, cx| {
            view.dismiss_menu(window, cx);
            view.open_rail_row_menu("local", target, point(px(100.), px(100.)), window, cx);
        })
    });
    draw(cx);
}
fn click(cx: &mut VisualTestContext, selector: &'static str) {
    let position = cx.debug_bounds(selector).unwrap().center();
    cx.simulate_click(position, Modifiers::default());
    draw(cx);
}

// Keep the acknowledgement under test control instead of draining the wait's
// timers, as simulate_click does with GPUI's deterministic scheduler.
fn click_before_timers(cx: &mut VisualTestContext, selector: &'static str) {
    let position = cx.debug_bounds(selector).unwrap().center();
    cx.update(|window, cx| {
        window.dispatch_event(
            MouseDownEvent {
                position,
                modifiers: Modifiers::default(),
                button: MouseButton::Left,
                click_count: 1,
                first_mouse: false,
            }
            .to_platform_input(),
            cx,
        );
        window.dispatch_event(
            MouseUpEvent {
                position,
                modifiers: Modifiers::default(),
                button: MouseButton::Left,
                click_count: 1,
            }
            .to_platform_input(),
            cx,
        );
    });
}

#[gpui::test]
fn row_menu_width_rows_and_detach_scope(cx: &mut TestAppContext) {
    let _ipad = crate::ipad_ui::ipad_layout();
    let (view, cx) = fixture(cx);
    for target in [
        None,
        Some(NavigationTarget::Workspace("w0")),
        Some(NavigationTarget::Pane("p0")),
    ] {
        open(&view, cx, target.clone());
        view.read_with(cx, |view, _| {
            assert!(matches!(view.menu.page, Some(Page::RowMenu(_))))
        });
        assert_eq!(cx.debug_bounds("menu-panel").unwrap().size.width, px(240.));
        for selector in [
            "ipad-rail-row-menu-Open",
            "ipad-rail-row-menu-NewTab",
            "ipad-rail-row-menu-OpenChat",
        ] {
            assert_eq!(cx.debug_bounds(selector).unwrap().size.height, px(48.));
        }
        assert_eq!(
            cx.debug_bounds("ipad-rail-row-menu-Detach").is_some(),
            target.is_none()
        );
    }
}

#[gpui::test]
fn row_menu_open_and_keyboard_actions_target_the_captured_row(cx: &mut TestAppContext) {
    let _ipad = crate::ipad_ui::ipad_layout();
    let (view, cx) = fixture(cx);
    open(&view, cx, Some(NavigationTarget::Workspace("w1")));
    click(cx, "ipad-rail-row-menu-Open");
    view.read_with(cx, |view, _| {
        assert_eq!(
            view.pending_navigation,
            Some(NavigationTarget::Workspace("w1".into()))
        )
    });
    open(&view, cx, Some(NavigationTarget::Pane("p0")));
    cx.simulate_keystrokes("down enter");
    view.read_with(cx, |view, _| {
        assert!(view.menu.page.is_none());
        assert_eq!(
            view.pending_navigation,
            Some(NavigationTarget::Pane("p0".into()))
        );
    });
}

#[gpui::test]
fn row_menu_detach_runs_the_host_command_and_dismisses(cx: &mut TestAppContext) {
    let _ipad = crate::ipad_ui::ipad_layout();
    let (view, cx) = fixture(cx);
    open(&view, cx, None);
    click(cx, "ipad-rail-row-menu-Detach");
    view.read_with(cx, |view, _| {
        assert_eq!(view.live.status, crate::state::ConnectionStatus::Detached);
        assert!(view.endpoints[0].connection.handle.is_none());
        assert!(view.menu.page.is_none());
        assert!(view.menu.row_menu.is_none());
    });
}

#[gpui::test]
fn row_menu_rejects_replaced_boot_and_missing_targets(cx: &mut TestAppContext) {
    let _ipad = crate::ipad_ui::ipad_layout();
    let (view, cx) = fixture(cx);
    for replaced_boot in [true, false] {
        open(&view, cx, Some(NavigationTarget::Workspace("w1")));
        view.update(cx, |view, _| {
            view.pending_navigation = None;
            let snapshot = Arc::make_mut(view.live.snapshot.as_mut().unwrap());
            if replaced_boot {
                snapshot.boot_id.push_str("-new");
            } else {
                snapshot
                    .workspaces
                    .retain(|workspace| workspace.workspace_id != "w1");
            }
        });
        click(cx, "ipad-rail-row-menu-Open");
        view.read_with(cx, |view, _| {
            assert!(view.pending_navigation.is_none());
            assert!(view.menu.page.is_none());
        });
    }
}

#[test]
fn row_menu_header_and_actions_are_translated() {
    use crate::i18n::leak_tests::{assert_translated, uncut, under_en_xa};
    under_en_xa("rail row menu", |cx, recorder| {
        let _ipad = crate::ipad_ui::ipad_layout();
        let (view, cx) = fixture(cx);
        let mut seen = Vec::new();
        for target in [
            None,
            Some(NavigationTarget::Workspace("w0")),
            Some(NavigationTarget::Pane("p0")),
        ] {
            open(&view, cx, target);
            seen.extend(recorder.take());
            assert_translated(
                "rail row menu",
                &uncut(seen.clone()),
                &["Local".into(), "herdr".into(), "Claude Code".into()],
            );
        }
    });
}

#[gpui::test]
fn row_menu_new_tab_waits_for_acknowledged_workspace_focus(cx: &mut TestAppContext) {
    use crate::{connection::ConnectionBridge, terminal::InputTarget, window::MockPeer};
    use herdr_client::protocol::{ClientMessage, ClientPaneInputEvent};
    let _ipad = crate::ipad_ui::ipad_layout();
    let mut peer = MockPeer::advertising(&["workspace.focus", "tab.create"]);
    let (view, cx) = fixture(cx);
    view.update(cx, |view, cx| {
        peer.prepare(view, cx);
        // Drive focus acknowledgement here; the shared mock's surface barrier
        // protocol is covered separately by the connection tests.
        view.live.supports_surface = false;
        let listing = Arc::make_mut(view.live.snapshot.as_mut().unwrap());
        let mut other = listing.workspaces[0].clone();
        other.workspace_id = "second".into();
        other.focused = false;
        listing.workspaces.push(other);
        assert!(view.input_ready());
    });
    open(&view, cx, Some(NavigationTarget::Workspace("second")));
    click_before_timers(cx, "ipad-rail-row-menu-NewTab");
    let focus = peer.request();
    assert_eq!(focus["method"], "workspace.focus");
    let id = focus["id"].as_str().unwrap();
    peer.respond(
        "boot-v1",
        id,
        &serde_json::json!({"id": id, "result": {"type":"ok"}}),
    );
    let sentinel = |peer: &mut MockPeer| {
        ConnectionBridge::send_input(
            &peer.client.handle,
            "boot-v1",
            &InputTarget::Pane("w1:p1".into()),
            ClientPaneInputEvent::TextCommit("sentinel".into()),
        )
        .unwrap();
        loop {
            match peer.receive() {
                ClientMessage::ClientShellEndpointRequest { request, .. } => {
                    let request: serde_json::Value = serde_json::from_str(&request).unwrap();
                    assert_ne!(request["method"], "tab.create");
                }
                ClientMessage::ClientShellPaneInput { .. } => break,
                _ => {}
            }
        }
    };
    sentinel(&mut peer);
    view.update(cx, |view, _| {
        Arc::make_mut(view.live.snapshot.as_mut().unwrap()).focused_workspace_id =
            Some("second".into());
        view.activation_deadline = None;
        assert!(
            view.input_ready(),
            "focused fixture must remain input-ready"
        );
    });
    cx.executor()
        .advance_clock(std::time::Duration::from_millis(100));
    cx.run_until_parked();
    view.read_with(cx, |view, _| {
        // Command::Tab honors the default ask-for-name setting.
        assert_eq!(
            view.menu.page,
            Some(Page::Dialog(crate::menu::page::WorkspaceAction::NewTab))
        );
        assert_eq!(
            view.live
                .snapshot
                .as_ref()
                .unwrap()
                .focused_workspace_id
                .as_deref(),
            Some("second")
        );
    });
}

#[gpui::test]
fn row_menu_open_chat_falls_back_to_the_group_chat_entry(cx: &mut TestAppContext) {
    let _ipad = crate::ipad_ui::ipad_layout();
    let peer = crate::window::MockPeer::new();
    let (view, cx) = fixture(cx);
    view.update(cx, |view, cx| peer.prepare(view, cx));
    open(&view, cx, None);
    click(cx, "ipad-rail-row-menu-OpenChat");
    view.read_with(cx, |view, _| assert_eq!(view.menu.page, Some(Page::Group)));
}

#[gpui::test]
fn row_menu_new_tab_uses_the_existing_creation_command(cx: &mut TestAppContext) {
    let _ipad = crate::ipad_ui::ipad_layout();
    let mut peer = crate::window::MockPeer::advertising(&["tab.create"]);
    let (view, cx) = fixture(cx);
    view.update(cx, |view, cx| {
        peer.prepare(view, cx);
        view.settings.shared = Some(
            crate::herdr_settings::Settings::parse_text("[ui]\nprompt_new_tab_name = false\n")
                .unwrap(),
        );
    });
    open(&view, cx, None);
    click(cx, "ipad-rail-row-menu-NewTab");
    let create = peer.request();
    assert_eq!(create["method"], "tab.create");
    assert_eq!(create["params"]["workspace_id"], "w1");
}

#[gpui::test]
fn row_menu_wait_cancels_for_a_replaced_host_or_later_navigation(cx: &mut TestAppContext) {
    let _ipad = crate::ipad_ui::ipad_layout();
    for replaced_host in [true, false] {
        let peer = crate::window::MockPeer::advertising(&["workspace.focus", "tab.create"]);
        let (view, cx) = fixture(cx);
        view.update(cx, |view, cx| {
            peer.prepare(view, cx);
            let listing = Arc::make_mut(view.live.snapshot.as_mut().unwrap());
            let mut other = listing.workspaces[0].clone();
            other.workspace_id = "second".into();
            other.focused = false;
            listing.workspaces.push(other);
        });
        open(&view, cx, Some(NavigationTarget::Workspace("second")));
        click_before_timers(cx, "ipad-rail-row-menu-NewTab");
        view.update(cx, |view, _| {
            if replaced_host {
                view.endpoints[0].generation += 1;
            } else {
                view.pending_navigation = Some(NavigationTarget::Workspace("w1".into()));
            }
            Arc::make_mut(view.live.snapshot.as_mut().unwrap()).focused_workspace_id =
                Some("second".into());
            view.activation_deadline = None;
            assert!(view.input_ready());
        });
        cx.run_until_parked();
        view.read_with(cx, |view, _| assert!(view.menu.page.is_none()));
    }
}

#[gpui::test]
fn row_menu_open_chat_focuses_the_existing_native_chat(cx: &mut TestAppContext) {
    use herdr_chat_protocol::{Cursor, EndpointId, ServerMessage, Transcript, VERSION};
    let _ipad = crate::ipad_ui::ipad_layout();
    let (view, cx) = fixture(cx);
    let endpoint: EndpointId = "0123456789abcdef0123456789abcdef".parse().unwrap();
    view.update(cx, |view, _| {
        let target = crate::chat::Target {
            endpoint: endpoint.clone(),
            existing: None,
            pane_id: "p0".into(),
            route: crate::chat::Route::Local,
            surface: crate::chat::Surface::Herdr,
        };
        view.chats
            .reconcile(vec![target], std::time::Instant::now(), &mut |_, _, _| {
                Err(crate::chat::Error::NotAttached)
            });
        let chat = view.chats.pane_mut("p0").unwrap();
        chat.session
            .receive(ServerMessage::Welcome {
                agent: "Claude".into(),
                pane: Some("p0".into()),
                version: VERSION,
            })
            .unwrap();
        chat.session
            .receive(ServerMessage::Snapshot {
                cursor: Cursor::new_epoch(),
                transcript: Box::new(Transcript::new("Claude")),
            })
            .unwrap();
        assert!(chat.available());
        assert!(view.chats.set_raw("p0", true));
        Arc::make_mut(view.live.snapshot.as_mut().unwrap()).agents[0]
            .tokens
            .push((herdr_chat_protocol::TOKEN.into(), endpoint.to_string()));
    });
    open(&view, cx, Some(NavigationTarget::Workspace("w0")));
    click(cx, "ipad-rail-row-menu-OpenChat");
    view.read_with(cx, |view, _| {
        assert_eq!(
            view.pending_navigation,
            Some(NavigationTarget::Pane("p0".into()))
        );
        assert!(view.menu.page.is_none());
        assert!(!view.chats.raw("p0"));
    });
}

#[gpui::test]
fn row_menu_rejects_changed_selection_generation_and_agent_membership(cx: &mut TestAppContext) {
    let _ipad = crate::ipad_ui::ipad_layout();
    let (view, cx) = fixture(cx);
    for change in 0..3 {
        open(&view, cx, Some(NavigationTarget::Pane("p0")));
        view.update(cx, |view, _| {
            view.pending_navigation = None;
            match change {
                0 => view.selection_epoch += 1,
                1 => view.endpoints[0].generation += 1,
                _ => Arc::make_mut(view.live.snapshot.as_mut().unwrap()).agents[0]
                    .tab_id
                    .push_str("-moved"),
            }
        });
        click(cx, "ipad-rail-row-menu-Open");
        view.read_with(cx, |view, _| {
            assert!(view.pending_navigation.is_none());
            assert!(view.menu.page.is_none());
        });
    }
}

#[gpui::test]
fn a_long_press_on_each_ipad_rail_row_opens_the_new_menu(cx: &mut TestAppContext) {
    let _ipad = crate::ipad_ui::ipad_layout();
    let (view, cx) = cx.add_window_view(|window, cx| {
        let mut view = fixture_window(window, cx);
        view.live.snapshot = Some(Arc::new(snapshot(2)));
        view.sidebar_visible = false;
        view
    });
    cx.simulate_resize(size(px(1194.), px(834.)));
    for selector in [
        "rail-host-local",
        "rail-workspace-local-w0",
        "rail-agent-local-p0",
    ] {
        draw(cx);
        let position = cx.debug_bounds(selector).unwrap().center();
        cx.update(|window, cx| {
            window.dispatch_event(
                LongPressEvent {
                    phase: TouchPhase::Started,
                    start_position: position,
                    position,
                }
                .to_platform_input(),
                cx,
            )
        });
        view.read_with(cx, |view, _| {
            assert!(matches!(view.menu.page, Some(Page::RowMenu(_))));
            assert!(!view.menu.opening_right_click);
        });
        cx.simulate_keystrokes("escape");
        view.read_with(cx, |view, _| assert!(view.menu.page.is_none()));
    }
}

#[gpui::test]
fn row_menu_keeps_its_content_while_leaving_and_resets_its_live_target(cx: &mut TestAppContext) {
    let _ipad = crate::ipad_ui::ipad_layout();
    let (view, cx) = cx.add_window_view(|window, cx| {
        let mut view = fixture_window(window, cx);
        view.live.snapshot = Some(Arc::new(snapshot(2)));
        view.sidebar_visible = false;
        view
    });
    cx.simulate_resize(size(px(1194.), px(834.)));
    open(&view, cx, None);
    assert!(cx.debug_bounds("ipad-rail-row-menu-Detach").is_some());
    // Hold the exit at its beginning independently of how long drawing takes.
    let closing = crate::motion::now() + std::time::Duration::from_secs(3600);
    cx.update(|window, cx| {
        view.update(cx, |view, cx| {
            view.menu.appear.live = true;
            view.tick_menu_appear_at(closing);
            view.dismiss_menu(window, cx);
            view.tick_menu_appear_at(closing);
        })
    });
    draw(cx);
    for selector in [
        "menu-leaving-shield",
        "ipad-rail-row-menu-header",
        "ipad-rail-row-menu-Open",
        "ipad-rail-row-menu-Detach",
    ] {
        assert!(cx.debug_bounds(selector).is_some(), "{selector}");
    }
    view.read_with(cx, |view, _| {
        assert!(view.menu.page.is_none());
        assert!(view.menu.row_menu.is_none());
        assert!(view.menu.appear.leaving(closing).is_some());
    });
    view.update(cx, |view, _| {
        view.tick_menu_appear_at(closing + crate::motion::LEAVE);
    });
    draw(cx);
    assert!(cx.debug_bounds("menu-leaving-shield").is_none());
    assert!(cx.debug_bounds("ipad-rail-row-menu-header").is_none());
    view.update(cx, |view, _| view.menu.appear.live = false);
    open(&view, cx, Some(NavigationTarget::Workspace("w1")));
    assert!(cx.debug_bounds("ipad-rail-row-menu-Open").is_some());
    assert!(cx.debug_bounds("ipad-rail-row-menu-Detach").is_none());
    assert!(cx.debug_bounds("menu-leaving-shield").is_none());
    view.read_with(cx, |view, _| {
        assert_eq!(
            view.menu.page,
            Some(Page::RowMenu(crate::menu::row_menu::RowTarget::Space))
        );
        assert!(view.menu.row_menu.is_some());
    });
}

#[gpui::test]
fn row_menu_workspace_actions_replace_an_older_deferred_navigation(cx: &mut TestAppContext) {
    let _ipad = crate::ipad_ui::ipad_layout();
    for (selector, page) in [
        (
            "ipad-rail-row-menu-NewTab",
            Page::Dialog(crate::menu::page::WorkspaceAction::NewTab),
        ),
        ("ipad-rail-row-menu-OpenChat", Page::Group),
    ] {
        let peer = crate::window::MockPeer::new();
        let (view, cx) = fixture(cx);
        view.update(cx, |view, cx| {
            peer.prepare(view, cx);
            let listing = Arc::make_mut(view.live.snapshot.as_mut().unwrap());
            let mut other = listing.workspaces[0].clone();
            other.workspace_id = "second".into();
            other.focused = false;
            listing.workspaces.push(other);
            view.pending_navigation = Some(NavigationTarget::Workspace("second".into()));
            assert!(view.input_ready());
        });
        open(&view, cx, Some(NavigationTarget::Workspace("w1")));
        click_before_timers(cx, selector);
        view.read_with(cx, |view, _| {
            assert!(
                view.pending_navigation.is_none(),
                "{selector} must replace the older request to open another workspace"
            );
            assert_eq!(view.menu.page, Some(page));
            assert_eq!(
                view.live
                    .snapshot
                    .as_ref()
                    .unwrap()
                    .focused_workspace_id
                    .as_deref(),
                Some("w1")
            );
        });
    }
}
