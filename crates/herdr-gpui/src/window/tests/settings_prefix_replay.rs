use super::*;
use crate::{
    config::ShortcutStyle,
    menu::Page,
    settings_window::{self, SheetFixture},
    window::MockPeer,
};
use core::prelude::v1::test;
use gpui::*;
use herdr_client::protocol::{
    ClientMessage, ClientPaneInputEvent, ClientShellCommand, ClientShellCommandAction,
};

fn open(
    cx: &mut TestAppContext,
    fail: bool,
) -> (Entity<HerdrWindow>, SheetFixture, &mut VisualTestContext) {
    let (view, cx) = cx.add_window_view(|window, cx| {
        crate::bind_keys(cx);
        let mut view = fixture_window(window, cx);
        view.config.shortcut_style = ShortcutStyle::HerdrPrefix;
        view
    });
    cx.simulate_resize(size(px(1194.), px(834.)));
    let fixture = cx.update(|_, cx| SheetFixture::prepare(view.downgrade(), fail, cx));
    cx.update(|_, cx| settings_window::open_at(view.downgrade(), None, cx));
    cx.run_until_parked();
    cx.update(|window, cx| window.draw(cx).clear(cx));
    (view, fixture, cx)
}

fn chord(window: &mut Window, cx: &mut App, key: &str) {
    assert!(window.dispatch_keystroke(Keystroke::parse("ctrl-b").unwrap(), cx));
    assert!(window.dispatch_keystroke(Keystroke::parse(key).unwrap(), cx));
}

#[gpui::test]
fn dirty_sheet_replays_the_chord_once_after_saving_and_returning_focus(cx: &mut TestAppContext) {
    let _ipad = crate::ipad_ui::ipad_layout();
    let (view, fixture, cx) = open(cx, false);
    cx.update(|window, cx| {
        fixture.dirty(cx);
        chord(window, cx, "b");
        let view = view.read(cx);
        assert_eq!(view.menu.page, Some(Page::Settings));
        assert!(view.sidebar_visible, "the command must wait for dismissal");
        assert!(view.menu.settings_prefix.is_some());
    });
    cx.run_until_parked();
    assert_eq!(*fixture.writes.lock().unwrap(), ["Nord"]);
    view.read_with(cx, |view, _| {
        assert!(view.menu.page.is_none());
        assert!(view.menu.settings_prefix.is_none());
        assert!(!view.prefix_armed);
        assert!(!view.sidebar_visible, "the toggle ran exactly once");
    });
    cx.update(|window, cx| assert!(view.read(cx).focus.is_focused(window)));
    cx.run_until_parked();
    assert!(!view.read_with(cx, |view, _| view.sidebar_visible));
}

#[gpui::test]
fn clean_sheet_replays_help_into_the_existing_keybinds_page(cx: &mut TestAppContext) {
    let _ipad = crate::ipad_ui::ipad_layout();
    let (view, fixture, cx) = open(cx, false);
    cx.update(|window, cx| chord(window, cx, "?"));
    cx.run_until_parked();
    view.read_with(cx, |view, cx| {
        assert_eq!(view.menu.page, Some(Page::Keybinds));
        assert!(view.menu.settings.is_none());
        assert!(view.menu.settings_prefix.is_none());
        assert!(
            view.menu
                .keybinds_search
                .as_ref()
                .unwrap()
                .read(cx)
                .text()
                .is_empty()
        );
    });
    assert!(fixture.writes.lock().unwrap().is_empty());
    cx.update(|window, cx| {
        assert!(
            view.read(cx)
                .menu
                .keybinds_search
                .as_ref()
                .unwrap()
                .read(cx)
                .focus
                .is_focused(window)
        );
    });
}

#[gpui::test]
fn a_failed_save_keeps_the_sheet_open_and_discards_the_chord(cx: &mut TestAppContext) {
    let _ipad = crate::ipad_ui::ipad_layout();
    let (view, fixture, cx) = open(cx, true);
    cx.update(|window, cx| {
        fixture.dirty(cx);
        chord(window, cx, "b");
    });
    cx.run_until_parked();
    assert_eq!(*fixture.writes.lock().unwrap(), ["Nord"]);
    view.read_with(cx, |view, _| {
        assert_eq!(view.menu.page, Some(Page::Settings));
        assert!(view.sidebar_visible);
        assert!(view.menu.settings_prefix.is_none());
    });
    cx.update(|window, cx| {
        window.draw(cx).clear(cx);
        assert!(fixture.focused(window, cx));
    });
    assert!(cx.debug_bounds("ipad-settings-sheet-error").is_some());
    // A later ordinary close attempt retries the save without retaining an action.
    view.update_in(cx, |view, window, cx| view.dismiss_menu(window, cx));
    cx.run_until_parked();
    view.read_with(cx, |view, _| {
        assert!(view.sidebar_visible);
        assert!(view.menu.settings_prefix.is_none());
        assert_eq!(view.menu.page, Some(Page::Settings));
    });
}

#[gpui::test]
fn duplicate_dismissals_do_not_replay_a_toggle_twice(cx: &mut TestAppContext) {
    let _ipad = crate::ipad_ui::ipad_layout();
    let (view, _, cx) = open(cx, false);
    cx.update(|window, cx| {
        chord(window, cx, "b");
        view.update(cx, |view, cx| view.dismiss_menu(window, cx));
    });
    cx.run_until_parked();
    view.read_with(cx, |view, _| {
        assert!(view.menu.page.is_none());
        assert!(view.menu.settings_prefix.is_none());
        assert!(!view.sidebar_visible);
    });
}

#[gpui::test]
fn reopening_the_same_editor_fences_the_old_chord(cx: &mut TestAppContext) {
    let _ipad = crate::ipad_ui::ipad_layout();
    let (view, fixture, cx) = open(cx, false);
    let generation = fixture
        .model
        .read_with(cx, |view, _| view.sheet_generation());
    cx.update(|window, cx| {
        chord(window, cx, "b");
        settings_window::open_at(view.downgrade(), None, cx);
    });
    cx.run_until_parked();
    assert_ne!(
        fixture
            .model
            .read_with(cx, |view, _| view.sheet_generation()),
        generation
    );
    view.read_with(cx, |view, _| {
        assert_eq!(view.menu.page, Some(Page::Settings));
        assert_eq!(view.menu.settings.as_ref(), Some(&fixture.model));
        assert!(view.menu.settings_prefix.is_none());
        assert!(view.sidebar_visible);
    });
    view.update_in(cx, |view, window, cx| view.dismiss_menu(window, cx));
    cx.run_until_parked();
    assert!(view.read_with(cx, |view, _| view.sidebar_visible));
}

#[gpui::test]
fn a_changed_target_discards_the_chord_after_the_sheet_closes(cx: &mut TestAppContext) {
    let _ipad = crate::ipad_ui::ipad_layout();
    let (view, _, cx) = open(cx, false);
    cx.update(|window, cx| {
        chord(window, cx, "b");
        view.update(cx, |view, _| view.selection_epoch += 1);
    });
    cx.run_until_parked();
    view.read_with(cx, |view, _| {
        assert!(view.menu.page.is_none());
        assert!(view.menu.settings_prefix.is_none());
        assert!(view.sidebar_visible);
    });
}

#[gpui::test]
fn one_close_attempt_keeps_only_its_first_chord(cx: &mut TestAppContext) {
    let _ipad = crate::ipad_ui::ipad_layout();
    let (view, _, cx) = open(cx, false);
    cx.update(|window, cx| {
        chord(window, cx, "b");
        chord(window, cx, "?");
    });
    cx.run_until_parked();
    view.read_with(cx, |view, _| {
        assert!(view.menu.page.is_none());
        assert!(view.menu.settings_prefix.is_none());
        assert!(!view.sidebar_visible);
    });
}

fn read_marker(peer: &mut MockPeer, marker: &str) {
    loop {
        match peer.receive() {
            ClientMessage::ClientShellPaneInput { pane_id, events } => {
                assert_eq!(pane_id, "w1:p1");
                assert_eq!(events, [ClientPaneInputEvent::TextCommit(marker.into())]);
                return;
            }
            ClientMessage::ClientShellEndpointRequest { request, .. } => {
                let request: serde_json::Value = serde_json::from_str(&request).unwrap();
                assert_ne!(request["method"], "command.invoke", "before {marker}");
            }
            _ => {}
        }
    }
}

#[gpui::test]
fn a_custom_chord_reaches_the_daemon_once_after_the_draft_is_saved(cx: &mut TestAppContext) {
    let _ipad = crate::ipad_ui::ipad_layout();
    let mut peer = MockPeer::advertising(&["command.invoke"]);
    let (view, cx) = cx.add_window_view(|window, cx| {
        let mut view = fixture_window(window, cx);
        peer.prepare(&mut view, cx);
        view.config.shortcut_style = ShortcutStyle::HerdrPrefix;
        Arc::make_mut(view.live.snapshot.as_mut().unwrap()).commands = vec![ClientShellCommand {
            command_id: "plugin.y".into(),
            description: None,
            action: ClientShellCommandAction::PluginAction,
            binding_label: "prefix+y".into(),
            binding_labels: vec!["prefix+y".into()],
        }];
        view
    });
    cx.simulate_resize(size(px(1194.), px(834.)));
    let fixture = cx.update(|_, cx| SheetFixture::prepare(view.downgrade(), false, cx));
    cx.update(|_, cx| settings_window::open_at(view.downgrade(), None, cx));
    cx.run_until_parked();
    cx.update(|window, cx| {
        window.draw(cx).clear(cx);
        view.update(cx, |view, cx| {
            let requested = view.options.surface_size;
            let surface = Arc::make_mut(view.live.surface.as_mut().unwrap());
            surface.frame.width = requested.cols;
            surface.frame.height = requested.rows;
            assert!(view.flush_pending_input(cx));
        });
        fixture.dirty(cx);
        chord(window, cx, "y");
        assert_eq!(view.read(cx).menu.page, Some(Page::Settings));
        peer.client
            .handle
            .send_input(
                "boot-v1",
                "w1:p1",
                [ClientPaneInputEvent::TextCommit("before save".into())],
            )
            .unwrap();
    });
    read_marker(&mut peer, "before save");
    cx.run_until_parked();
    assert_eq!(*fixture.writes.lock().unwrap(), ["Nord"]);
    view.read_with(cx, |view, _| {
        assert!(view.menu.page.is_none());
        assert!(view.menu.settings_prefix.is_none());
        assert!(view.local_error.is_none());
    });
    let request = peer.request();
    assert_eq!(request["method"], "command.invoke");
    assert_eq!(request["params"]["command_id"], "plugin.y");
    let id = request["id"].as_str().unwrap();
    peer.respond(
        "boot-v1",
        id,
        &serde_json::json!({"id": id, "result": {"type": "ok"}}),
    );
    view.update_in(cx, |view, window, cx| view.dismiss_menu(window, cx));
    cx.run_until_parked();
    peer.client
        .handle
        .send_input(
            "boot-v1",
            "w1:p1",
            [ClientPaneInputEvent::TextCommit("after replay".into())],
        )
        .unwrap();
    read_marker(&mut peer, "after replay");
}
