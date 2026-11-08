use super::*;
use crate::{
    config::ShortcutStyle,
    keymap::{DaemonKeys, Keymap},
    window::MockPeer,
};
use core::prelude::v1::test;
use gpui::*;
use sheprd_client::protocol::{
    ClientKeyCode, ClientKeyKind, ClientMessage, ClientPaneInputEvent, ClientShellCommand,
    ClientShellCommandAction,
};

fn open<'a>(
    cx: &'a mut TestAppContext,
    style: ShortcutStyle,
    prefix: &str,
) -> (Entity<SheprdWindow>, MockPeer, &'a mut VisualTestContext) {
    let peer = MockPeer::new();
    let (view, cx) = cx.add_window_view(|window, cx| {
        let mut view = fixture_window(window, cx);
        peer.prepare(&mut view, cx);
        view.config.shortcut_style = style;
        let table = format!("prefix = '{prefix}'").parse().unwrap();
        view.config.keybindings = Keymap::with_overrides(
            &Default::default(),
            &Default::default(),
            &DaemonKeys::from_table(Some(&table)),
        )
        .unwrap();
        window.focus(&view.focus, cx);
        view
    });
    cx.simulate_resize(size(px(1194.), px(834.)));
    draw(cx);
    view.update(cx, |view, cx| {
        // A mock peer does not answer the resize caused by the native layout.
        let requested = view.options.surface_size;
        let surface = Arc::make_mut(view.live.surface.as_mut().unwrap());
        surface.frame.width = requested.cols;
        surface.frame.height = requested.rows;
        assert!(view.flush_pending_input(cx));
    });
    (view, peer, cx)
}

fn draw(cx: &mut VisualTestContext) {
    cx.update(|window, cx| window.draw(cx).clear(cx));
}

fn next_input(peer: &mut MockPeer) -> Vec<ClientPaneInputEvent> {
    loop {
        match peer.receive() {
            ClientMessage::ClientShellPaneInput { pane_id, events } => {
                assert_eq!(pane_id, "w1:p1");
                return events;
            }
            ClientMessage::ClientShellEndpointRequest { request, .. } => {
                let request: serde_json::Value = serde_json::from_str(&request).unwrap();
                assert_ne!(request["method"], "command.invoke");
            }
            _ => {}
        }
    }
}

fn control(peer: &mut MockPeer, character: char) {
    assert!(matches!(
        next_input(peer).as_slice(),
        [ClientPaneInputEvent::Key {
            code: ClientKeyCode::Char(code),
            modifiers: 2,
            kind: ClientKeyKind::Press,
            ..
        }] if *code == character
    ));
}

#[gpui::test]
fn ipad_prefix_keys_reach_the_terminal_without_arming_or_showing_the_indicator(
    cx: &mut TestAppContext,
) {
    let _ipad = crate::ipad_ui::ipad_layout();
    let (view, mut peer, cx) = open(cx, ShortcutStyle::Ipad, "ctrl+b");
    for (prefix, key, character) in [("ctrl+b", "ctrl-b", 'b'), ("ctrl+a", "ctrl-a", 'a')] {
        view.update(cx, |view, _| {
            let table = format!("prefix = '{prefix}'").parse().unwrap();
            view.config.keybindings = Keymap::with_overrides(
                &Default::default(),
                &Default::default(),
                &DaemonKeys::from_table(Some(&table)),
            )
            .unwrap();
        });
        cx.simulate_keystrokes(key);
        control(&mut peer, character);
        draw(cx);
        assert!(!view.read_with(cx, |view, _| view.prefix_armed));
        assert!(cx.debug_bounds("prefix-armed").is_none());
        cx.simulate_keystrokes("b");
        assert_eq!(
            next_input(&mut peer),
            [ClientPaneInputEvent::TextCommit("b".into())]
        );
        assert!(view.read_with(cx, |view, _| view.sidebar_visible));
    }
}

#[gpui::test]
fn ipad_style_cannot_reach_a_prefixed_custom_command(cx: &mut TestAppContext) {
    let _ipad = crate::ipad_ui::ipad_layout();
    let (view, mut peer, cx) = open(cx, ShortcutStyle::Ipad, "ctrl+b");
    view.update(cx, |view, _| {
        Arc::make_mut(view.live.snapshot.as_mut().unwrap()).commands = vec![ClientShellCommand {
            command_id: "plugin.y".into(),
            description: None,
            action: ClientShellCommandAction::PluginAction,
            binding_label: "prefix+y".into(),
            binding_labels: vec!["prefix+y".into()],
        }];
    });
    cx.simulate_keystrokes("ctrl-b y");
    control(&mut peer, 'b');
    assert_eq!(
        next_input(&mut peer),
        [ClientPaneInputEvent::TextCommit("y".into())]
    );
    view.read_with(cx, |view, _| {
        assert!(!view.prefix_armed);
        assert!(view.local_error.is_none());
    });
}

#[gpui::test]
fn ipad_typing_prefix_remains_ordinary_text_in_a_menu(cx: &mut TestAppContext) {
    let _ipad = crate::ipad_ui::ipad_layout();
    let (view, _, cx) = open(cx, ShortcutStyle::Ipad, "@");
    view.update_in(cx, |view, window, cx| view.open_keybinds(window, cx));
    cx.run_until_parked();
    cx.simulate_keystrokes("@ @ b");
    draw(cx);
    view.read_with(cx, |view, cx| {
        assert!(!view.prefix_armed);
        assert_eq!(view.menu.page, Some(crate::menu::Page::Keybinds));
        assert_eq!(
            view.menu.keybinds_search.as_ref().unwrap().read(cx).text(),
            "@@b"
        );
        assert!(view.sidebar_visible);
    });
    assert!(cx.debug_bounds("prefix-armed").is_none());
}

#[gpui::test]
fn herdr_style_still_arms_cancels_runs_chords_and_passes_a_double_press(cx: &mut TestAppContext) {
    let _ipad = crate::ipad_ui::ipad_layout();
    let (view, mut peer, cx) = open(cx, ShortcutStyle::HerdrPrefix, "ctrl+b");
    cx.simulate_keystrokes("ctrl-b");
    draw(cx);
    assert!(view.read_with(cx, |view, _| view.prefix_armed));
    assert!(cx.debug_bounds("prefix-armed").is_some());
    cx.simulate_keystrokes("ctrl-b");
    control(&mut peer, 'b');
    assert!(!view.read_with(cx, |view, _| view.prefix_armed));
    cx.simulate_keystrokes("ctrl-b escape");
    assert!(!view.read_with(cx, |view, _| view.prefix_armed));
    cx.simulate_keystrokes("ctrl-b b");
    assert!(!view.read_with(cx, |view, _| view.sidebar_visible));
    draw(cx);
    assert!(cx.debug_bounds("prefix-armed").is_none());
    peer.client
        .handle
        .send_input(
            "boot-v1",
            "w1:p1",
            [ClientPaneInputEvent::TextCommit("sentinel".into())],
        )
        .unwrap();
    assert_eq!(
        next_input(&mut peer),
        [ClientPaneInputEvent::TextCommit("sentinel".into())]
    );
}

#[gpui::test]
fn desktop_keeps_prefix_behavior_even_with_ipad_style_saved(cx: &mut TestAppContext) {
    let _desktop = crate::ipad_ui::desktop();
    let (view, _, cx) = open(cx, ShortcutStyle::Ipad, "ctrl+b");
    cx.simulate_keystrokes("ctrl-b");
    assert!(view.read_with(cx, |view, _| view.prefix_armed));
    cx.simulate_keystrokes("b");
    assert!(!view.read_with(cx, |view, _| view.sidebar_visible));
}

#[gpui::test]
fn switching_to_ipad_style_abandons_an_old_arm_before_the_next_key(cx: &mut TestAppContext) {
    let _ipad = crate::ipad_ui::ipad_layout();
    let (view, mut peer, cx) = open(cx, ShortcutStyle::HerdrPrefix, "ctrl+b");
    cx.simulate_keystrokes("ctrl-b");
    view.update(cx, |view, _| {
        view.config.shortcut_style = ShortcutStyle::Ipad
    });
    cx.simulate_keystrokes("ctrl-b");
    control(&mut peer, 'b');
    draw(cx);
    assert!(!view.read_with(cx, |view, _| view.prefix_armed));
    assert!(cx.debug_bounds("prefix-armed").is_none());
}
