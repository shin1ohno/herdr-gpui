#![allow(clippy::unwrap_used)]
use super::*;
use crate::sidebar::layout_tests::{fixture_window, full_draw};
use core::prelude::v1::test;

fn draw(cx: &mut VisualTestContext) {
    cx.update(|window, cx| full_draw(window, cx).clear(cx));
}

#[gpui::test]
fn guide_touch_opens_without_settings_and_escape_dismisses(cx: &mut TestAppContext) {
    let _ipad = crate::ipad_ui::ipad_layout();
    let (view, cx) = cx.add_window_view(|window, cx| {
        let mut view = fixture_window(window, cx);
        view.config.shortcut_style = ShortcutStyle::Ipad;
        view.resize_mode = true;
        window.focus(&view.focus, cx);
        view
    });
    cx.simulate_resize(size(px(1194.), px(834.)));
    draw(cx);
    let button = cx.debug_bounds("status-keybinds").unwrap();
    cx.simulate_click(button.center(), Modifiers::default());
    draw(cx);
    assert!(cx.debug_bounds("shortcut-guide").is_some());
    view.read_with(cx, |view, _| {
        assert!(!view.prefix_armed);
        assert!(!view.resize_mode);
        assert!(view.menu.page.is_none());
    });
    cx.simulate_keystrokes("escape");
    draw(cx);
    assert!(cx.debug_bounds("shortcut-guide").is_none());
}

#[gpui::test]
fn guide_runs_touch_action_and_prefix_shows_next_keys(cx: &mut TestAppContext) {
    let _ipad = crate::ipad_ui::ipad_layout();
    let (view, cx) = cx.add_window_view(|window, cx| {
        let mut view = fixture_window(window, cx);
        view.config.shortcut_style = ShortcutStyle::HerdrPrefix;
        window.focus(&view.focus, cx);
        view
    });
    cx.simulate_resize(size(px(1194.), px(834.)));
    cx.simulate_keystrokes("ctrl-b");
    draw(cx);
    assert!(cx.debug_bounds("shortcut-guide").is_some());
    let before = view.read_with(cx, |view, _| view.sidebar_visible);
    let button = cx.debug_bounds("shortcut-guide-ToggleSidebar").unwrap();
    cx.simulate_click(button.center(), Modifiers::default());
    draw(cx);
    view.read_with(cx, |view, _| {
        assert!(!view.prefix_armed);
        assert_ne!(view.sidebar_visible, before);
    });
    assert!(cx.debug_bounds("shortcut-guide").is_none());
}

#[test]
fn guide_respects_unbinding_and_reassigned_prefix() {
    use crate::keymap::{Binding, DaemonKeys};
    let mut overrides = std::collections::BTreeMap::new();
    overrides.insert("split_right".into(), Binding::Many(Vec::new()));
    let table = "prefix = 'ctrl+a'\nnew_tab = 'prefix+q'".parse().unwrap();
    let keys = Keymap::with_overrides(
        &overrides,
        &Default::default(),
        &DaemonKeys::from_table(Some(&table)),
    )
    .unwrap();
    assert_eq!(
        binding(&keys, Command::SplitRight, ShortcutStyle::HerdrPrefix, true),
        None
    );
    assert_eq!(
        binding(&keys, Command::Tab, ShortcutStyle::HerdrPrefix, true),
        Some("q".into())
    );
    assert_eq!(keys.prefix_label().as_deref(), Some("ctrl-a"));
    overrides.insert("new_tab".into(), Binding::One("cmd-alt-y".into()));
    let keys =
        Keymap::with_overrides(&overrides, &Default::default(), &DaemonKeys::default()).unwrap();
    assert_eq!(
        binding(&keys, Command::Tab, ShortcutStyle::Ipad, false),
        Some("⌘⌥Y".into())
    );
    assert_eq!(
        binding(&keys, Command::Tab, ShortcutStyle::HerdrPrefix, true),
        None
    );
}

#[gpui::test]
fn guide_command_narrow_bounds_outside_dismissal_and_settings(cx: &mut TestAppContext) {
    let _ipad = crate::ipad_ui::ipad_layout();
    let (view, cx) = cx.add_window_view(|window, cx| {
        let mut view = fixture_window(window, cx);
        view.config.shortcut_style = ShortcutStyle::Ipad;
        window.focus(&view.focus, cx);
        view
    });
    for width in [1194., 500., 375., 320.] {
        cx.simulate_resize(size(px(width), px(834.)));
        view.update_in(cx, |view, window, cx| {
            view.command(Command::Keybinds, window, cx)
        });
        draw(cx);
        let guide = cx.debug_bounds("shortcut-guide").unwrap();
        assert!(
            guide.left() >= px(0.) && guide.right() <= px(width),
            "{width}: {guide:?}"
        );
        for selector in ["shortcut-guide-close", "shortcut-guide-settings"] {
            let button = cx.debug_bounds(selector).unwrap();
            assert!(
                button.left() >= guide.left() && button.right() <= guide.right(),
                "{width}: {selector} {button:?} outside {guide:?}"
            );
        }
        cx.simulate_click(point(px(width / 2.), px(60.)), Modifiers::default());
        draw(cx);
        assert!(cx.debug_bounds("shortcut-guide").is_none());
    }
    view.update_in(cx, |view, window, cx| {
        view.command(Command::Keybinds, window, cx)
    });
    draw(cx);
    let settings = cx.debug_bounds("shortcut-guide-settings").unwrap();
    cx.simulate_click(settings.center(), Modifiers::default());
    draw(cx);
    view.read_with(cx, |view, _| {
        assert_eq!(view.menu.page, Some(crate::menu::Page::Settings));
        assert!(!view.prefix_armed);
    });
}

#[gpui::test]
fn guide_hides_shadowed_custom_keys_and_stale_host_commands(cx: &mut TestAppContext) {
    use herdr_client::protocol::{ClientShellCommand, ClientShellCommandAction};
    use std::sync::Arc;
    let _ipad = crate::ipad_ui::ipad_layout();
    let (view, cx) = cx.add_window_view(fixture_window);
    view.update(cx, |view, cx| {
        view.live.status = crate::state::ConnectionStatus::Connected;
        let command = |label: &str| ClientShellCommand {
            command_id: "guide-test".into(),
            binding_label: label.into(),
            binding_labels: vec![label.into()],
            action: ClientShellCommandAction::Shell,
            description: Some("Test command".into()),
        };
        Arc::make_mut(view.live.snapshot.as_mut().unwrap()).commands = vec![command("prefix+b")];
        assert!(
            view.custom_guide_section(ShortcutStyle::HerdrPrefix, true, cx)
                .is_none()
        );
        Arc::make_mut(view.live.snapshot.as_mut().unwrap()).commands = vec![command("prefix+y")];
        assert!(
            view.custom_guide_section(ShortcutStyle::HerdrPrefix, true, cx)
                .is_some()
        );
        assert!(
            view.custom_guide_section(ShortcutStyle::Ipad, false, cx)
                .is_none()
        );
        view.live.status = crate::state::ConnectionStatus::Disconnected;
        assert!(
            view.custom_guide_section(ShortcutStyle::HerdrPrefix, true, cx)
                .is_none()
        );
    });
}

#[gpui::test]
fn guide_terminal_actions_follow_connection_state(cx: &mut TestAppContext) {
    let _ipad = crate::ipad_ui::ipad_layout();
    let (view, cx) = cx.add_window_view(|window, cx| {
        let mut view = fixture_window(window, cx);
        view.live.status = crate::state::ConnectionStatus::Connected;
        let listing = std::sync::Arc::make_mut(view.live.snapshot.as_mut().unwrap());
        listing.focused_pane_id = Some("p0".into());
        listing.panes.push(herdr_client::protocol::ClientShellPane {
            pane_id: "p0".into(),
            workspace_id: "w0".into(),
            tab_id: "t0".into(),
            label: None,
            cwd: None,
            foreground_cwd: None,
            focused: true,
            right_click_passthrough: false,
        });
        view.config.shortcut_style = ShortcutStyle::HerdrPrefix;
        view.toggle_shortcut_guide(cx);
        view
    });
    cx.simulate_resize(size(px(1194.), px(834.)));
    draw(cx);
    assert!(cx.debug_bounds("shortcut-guide-Tab").is_some());
    assert!(cx.debug_bounds("shortcut-guide-SplitRight").is_some());
    view.update(cx, |view, cx| {
        view.live.status = crate::state::ConnectionStatus::Disconnected;
        cx.notify();
    });
    draw(cx);
    assert!(cx.debug_bounds("shortcut-guide-SplitRight").is_none());
    assert!(cx.debug_bounds("shortcut-guide-Tab").is_none());
    assert!(cx.debug_bounds("shortcut-guide-ToggleSidebar").is_some());
}
