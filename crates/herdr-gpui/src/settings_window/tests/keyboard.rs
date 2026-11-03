use super::*;
use crate::{
    config::{DeviceSettings, KeybindingSource, ShortcutStyle, preferences::Preference},
    controls::Command,
    keymap::{Keymap, glyphs},
    settings_window::{
        controls::preferences::PreferenceIo,
        pages::keyboard::{KEYBOARD_ROWS, prefix_host, prefix_value, samples, shortcut_values},
    },
};

const PROFILE_ID: &str = "0123456789abcdef0123456789abcdef";
const HOST: &str = "ssh:0123456789abcdef0123456789abcdef";

pub(super) fn resolved_keys(text: &str) -> Result<Keymap> {
    let overrides = toml::from_str(text)?;
    Ok(Keymap::with_overrides(
        &overrides,
        &Default::default(),
        &Default::default(),
    )?)
}

fn page(cx: &mut TestAppContext) -> (Entity<SettingsWindow>, &mut VisualTestContext) {
    let main = cx.add_window(crate::sidebar::layout_tests::fixture_window);
    let weak = cx.update(|cx| main.update(cx, |_, _, cx| cx.weak_entity()).unwrap());
    cx.add_window_view(|_, cx| {
        let mut view = SettingsWindow::new(weak, cx);
        view.section = Section::Keyboard;
        view
    })
}

fn draw(cx: &mut VisualTestContext) {
    cx.update(|window, cx| crate::sidebar::layout_tests::full_draw(window, cx).clear(cx));
}

#[test]
fn keyboard_is_shown_after_tabs_only_in_the_ipad_layout() {
    let desktop = Section::shown().collect::<Vec<_>>();
    assert!(!desktop.contains(&Section::Keyboard));
    assert_eq!(desktop.as_slice(), Section::ALL);
    let _ipad = crate::ipad_ui::ipad_layout();
    let shown = Section::shown().collect::<Vec<_>>();
    let tabs = shown
        .iter()
        .position(|section| *section == Section::Tabs)
        .unwrap();
    assert_eq!(shown.get(tabs + 1), Some(&Section::Keyboard));
    assert_eq!(Section::Keyboard.group(), Some(Group::Behaviour));
    assert!(
        catalog::search("Keyboard")
            .iter()
            .any(|&index| catalog::CATALOG[index].section == Section::Keyboard)
    );
    let _desktop = crate::ipad_ui::desktop();
    assert!(!Section::shown().any(|section| section == Section::Keyboard));
    assert!(
        catalog::search("Keyboard")
            .iter()
            .all(|&index| catalog::CATALOG[index].section != Section::Keyboard)
    );
}

#[gpui::test]
fn keyboard_radio_saves_the_typed_style_and_keeps_drafts(cx: &mut TestAppContext) {
    let _ipad = crate::ipad_ui::ipad_layout();
    let (view, cx) = page(cx);
    let edits = Arc::new(Mutex::new(Vec::new()));
    let saved = edits.clone();
    view.update(cx, |view, _| {
        view.config.shortcut_style = ShortcutStyle::HerdrPrefix;
        view.layout_intent = Some(crate::config::LayoutMode::Orca);
        view.controls.preference_io = Some(PreferenceIo::recording(move |edit| {
            saved.lock().unwrap().push(edit)
        }));
    });
    cx.simulate_resize(size(px(1194.), px(1200.)));
    draw(cx);
    let radio = cx.debug_bounds("keyboard-style-ipad").unwrap();
    assert_eq!(radio.size.height, px(crate::ipad_ui::metrics::STYLE_ROW));
    cx.simulate_click(radio.center(), Default::default());
    cx.run_until_parked();
    assert_eq!(
        *edits.lock().unwrap(),
        [Preference::ShortcutStyle(ShortcutStyle::Ipad)]
    );
    view.read_with(cx, |view, _| {
        assert!(!view.busy());
        assert_eq!(view.layout_intent, Some(crate::config::LayoutMode::Orca));
    });
    view.update(cx, |view, cx| {
        view.config.shortcut_style = ShortcutStyle::Ipad;
        cx.notify();
    });
    draw(cx);
    let point = cx
        .debug_bounds("keyboard-style-herdr-prefix")
        .unwrap()
        .center();
    cx.simulate_click(point, Default::default());
    cx.run_until_parked();
    assert_eq!(
        edits.lock().unwrap().last(),
        Some(&Preference::ShortcutStyle(ShortcutStyle::HerdrPrefix))
    );
}

#[test]
fn keyboard_cells_and_samples_use_styled_resolved_bindings() -> Result<()> {
    let resolved =
        resolved_keys("new_tab = 'cmd-alt-t'\nsplit_right = 'cmd-alt-d'\nclose_pane = []\n")?;
    let keymap = &resolved;
    for &style in ShortcutStyle::ALL {
        for &(_, commands) in KEYBOARD_ROWS {
            if commands
                .iter()
                .all(|command| matches!(command, Command::TabNumber(_)))
            {
                continue;
            }
            let expected: Vec<_> = commands
                .iter()
                .map(|&command| {
                    keymap
                        .styled(command, style)
                        .map(glyphs)
                        .unwrap_or_else(|| "—".into())
                })
                .collect();
            let actual = shortcut_values(keymap, commands, style);
            if expected.iter().all(|value| value == "—") {
                assert_eq!(actual, ["—"]);
            } else {
                assert_eq!(actual, expected);
            }
        }
    }
    assert_eq!(
        shortcut_values(keymap, &[Command::Tab], ShortcutStyle::Ipad),
        ["⌘⌥T"]
    );
    assert_eq!(
        shortcut_values(keymap, &[Command::Tab], ShortcutStyle::HerdrPrefix),
        ["—"]
    );
    assert_eq!(
        shortcut_values(keymap, &[Command::ClosePane], ShortcutStyle::Ipad),
        ["—"]
    );
    assert!(samples(keymap, ShortcutStyle::Ipad).contains(&"⌘⌥T".to_owned()));
    assert!(samples(keymap, ShortcutStyle::Ipad).contains(&"⌘⌥D".to_owned()));
    assert_eq!(
        shortcut_values(
            &Keymap::default(),
            &[Command::ToggleSidebar],
            ShortcutStyle::HerdrPrefix
        ),
        ["⌃B b"]
    );
    Ok(())
}

#[test]
fn keyboard_numbered_tabs_fold_only_a_complete_resolved_range() -> Result<()> {
    let (_, commands) = KEYBOARD_ROWS
        .iter()
        .find(|(_, commands)| commands.contains(&Command::TabNumber(1)))
        .unwrap();
    assert_eq!(
        *commands,
        (1..=9).map(Command::TabNumber).collect::<Vec<_>>()
    );
    let keymap = Keymap::default();
    assert_eq!(
        shortcut_values(&keymap, commands, ShortcutStyle::HerdrPrefix),
        ["⌃B 1–9"]
    );
    // Use the command catalog name, rather than assuming it here.
    let name = crate::controls::COMMANDS
        .iter()
        .find(|info| info.command == Command::TabNumber(5))
        .unwrap()
        .name;
    let keymap = resolved_keys(&format!("{name} = []\n"))?;
    let values = shortcut_values(&keymap, commands, ShortcutStyle::HerdrPrefix);
    assert_eq!(values.len(), commands.len());
    assert_eq!(values[4], "—");
    assert_eq!(values[0], "⌃B 1");
    assert_eq!(values[8], "⌃B 9");
    Ok(())
}

#[test]
fn keyboard_prefix_override_and_source_toggle_are_honest() -> Result<()> {
    let keymap = resolved_keys("new_tab = 'ctrl-b'\n")?;
    assert_eq!(prefix_value(&keymap), "—");
    let mut config = Config::default();
    for endpoint in ["local", "cloud:box", "wsl:Ubuntu", "ssh:invalid"] {
        assert_eq!(config.toggle_keybinding_source(endpoint), None);
    }
    assert_eq!(
        config.toggle_keybinding_source(HOST),
        Some((PROFILE_ID.into(), KeybindingSource::Server))
    );
    config.devices.insert(
        PROFILE_ID.into(),
        DeviceSettings {
            keybindings: KeybindingSource::Server,
            ..Default::default()
        },
    );
    assert_eq!(
        config.toggle_keybinding_source(HOST),
        Some((PROFILE_ID.into(), KeybindingSource::Local))
    );
    Ok(())
}

#[gpui::test]
fn keyboard_prefix_names_the_host_only_for_an_applied_server_profile(cx: &mut TestAppContext) {
    let (owner, cx) = cx.add_window_view(crate::sidebar::layout_tests::fixture_window);
    owner.update(cx, |owner, cx| {
        owner.endpoints.push(crate::endpoint::Endpoint::new(
            HOST.into(),
            "studio-mac".into(),
            herdr_client::ConnectTarget::Ssh {
                target: "you@studio-mac".into(),
                session: "default".into(),
            },
            false,
        ));
        owner.selected_endpoint = 1;
        owner.config.devices.insert(
            PROFILE_ID.into(),
            DeviceSettings {
                keybindings: KeybindingSource::Server,
                ..Default::default()
            },
        );
        for profile in [None, Some("[keys"), Some("[keys]\nprefix = 'ctrl+a'\n")] {
            let mut snapshot = crate::sidebar::layout_tests::snapshot(2);
            snapshot.server_keybindings_toml = profile.map(str::to_owned);
            owner.live.snapshot = Some(Arc::new(snapshot));
            owner.sync_server_keymap(cx);
            let expected = profile
                .filter(|profile| profile.contains("ctrl+a"))
                .map(|_| "studio-mac");
            assert_eq!(prefix_host(owner), expected);
            if expected.is_some() {
                assert_eq!(prefix_value(owner.keymap()), "⌃A");
            }
        }
        owner.config.devices.clear();
        owner.sync_server_keymap(cx);
        assert_eq!(prefix_host(owner), None);
    });
}

#[gpui::test]
fn keyboard_local_switch_is_inert_and_rows_keep_their_touch_sizes(cx: &mut TestAppContext) {
    let _ipad = crate::ipad_ui::ipad_layout();
    let (view, cx) = page(cx);
    let edits = Arc::new(Mutex::new(Vec::new()));
    let saved = edits.clone();
    view.update(cx, |view, _| {
        view.controls.preference_io = Some(PreferenceIo::recording(move |edit| {
            saved.lock().unwrap().push(edit);
        }));
    });
    cx.simulate_resize(size(px(1194.), px(1600.)));
    draw(cx);
    assert_eq!(
        cx.debug_bounds("keyboard-prefix").unwrap().size.height,
        px(crate::ipad_ui::metrics::SETTINGS_ROW)
    );
    assert_eq!(
        cx.debug_bounds("keyboard-server").unwrap().size.height,
        px(crate::ipad_ui::metrics::SETTINGS_ROW)
    );
    let toggle = cx.debug_bounds("keyboard-server-toggle").unwrap();
    assert!(toggle.size.width >= px(44.) && toggle.size.height >= px(44.));
    cx.simulate_click(toggle.center(), Default::default());
    cx.run_until_parked();
    assert!(edits.lock().unwrap().is_empty());
    view.read_with(cx, |view, _| {
        assert!(!view.busy());
        assert!(view.error.is_none());
        assert!(view.config.devices.is_empty());
    });
    for &(message, _) in KEYBOARD_ROWS {
        assert_eq!(
            cx.debug_bounds(message).unwrap().size.height,
            px(crate::ipad_ui::metrics::SHORTCUT_ROW)
        );
    }
}

#[gpui::test]
fn keyboard_server_switch_uses_the_shared_choice_and_typed_save(cx: &mut TestAppContext) {
    let _ipad = crate::ipad_ui::ipad_layout();
    let (view, cx) = page(cx);
    let source = view.read_with(cx, |view, _| view.source.upgrade().unwrap());
    source.update(cx, |source, cx| {
        source.endpoints.push(crate::endpoint::Endpoint::new(
            HOST.into(),
            "studio-mac".into(),
            herdr_client::ConnectTarget::Ssh {
                target: "you@studio-mac".into(),
                session: "default".into(),
            },
            false,
        ));
        source.selected_endpoint = 1;
        cx.notify();
    });
    let edits = Arc::new(Mutex::new(Vec::new()));
    let saved = edits.clone();
    view.update(cx, |view, _| {
        view.controls.preference_io = Some(PreferenceIo::recording(move |edit| {
            saved.lock().unwrap().push(edit)
        }));
    });
    cx.simulate_resize(size(px(1194.), px(1600.)));
    for choice in [KeybindingSource::Server, KeybindingSource::Local] {
        if choice == KeybindingSource::Local {
            view.update(cx, |view, cx| {
                view.config.devices.insert(
                    PROFILE_ID.into(),
                    DeviceSettings {
                        keybindings: KeybindingSource::Server,
                        ..Default::default()
                    },
                );
                cx.notify();
            });
        }
        draw(cx);
        let point = cx.debug_bounds("keyboard-server-toggle").unwrap().center();
        cx.simulate_click(point, Default::default());
        cx.run_until_parked();
        assert_eq!(
            edits.lock().unwrap().last(),
            Some(&Preference::DeviceKeybindings {
                profile: PROFILE_ID.into(),
                source: choice
            })
        );
    }
}

#[gpui::test]
fn keyboard_partial_number_range_scrolls_inside_its_own_column(cx: &mut TestAppContext) {
    let _ipad = crate::ipad_ui::ipad_layout();
    let (view, cx) = page(cx);
    let name = crate::controls::COMMANDS
        .iter()
        .find(|info| info.command == Command::TabNumber(5))
        .unwrap()
        .name;
    let keys = resolved_keys(&format!("{name} = []\n")).unwrap();
    let source = view.read_with(cx, |view, _| view.source.upgrade().unwrap());
    source.update(cx, |source, cx| {
        source.config.keybindings = keys;
        cx.notify();
    });
    cx.simulate_resize(size(px(960.), px(1600.)));
    draw(cx);
    let cell = cx
        .debug_bounds("ipad-keyboard-row-tab-number-herdr-prefix")
        .unwrap();
    let other = cx
        .debug_bounds("ipad-keyboard-row-tab-number-ipad")
        .unwrap();
    let first = cx
        .debug_bounds("ipad-keyboard-row-tab-number-herdr-prefix-0")
        .unwrap();
    let last = cx
        .debug_bounds("ipad-keyboard-row-tab-number-herdr-prefix-8")
        .unwrap();
    assert!(
        last.right() > cell.right(),
        "the fixture must overflow its column"
    );
    let row = cx.debug_bounds("ipad-keyboard-row-tab-number").unwrap();
    assert_eq!(row.size.height, px(crate::ipad_ui::metrics::SHORTCUT_ROW));
    assert!(other.right() <= cell.left());
    cx.simulate_event(ScrollWheelEvent {
        position: cell.center(),
        delta: ScrollDelta::Pixels(point(px(-2000.), px(0.))),
        ..Default::default()
    });
    draw(cx);
    let after_first = cx
        .debug_bounds("ipad-keyboard-row-tab-number-herdr-prefix-0")
        .unwrap();
    let after_last = cx
        .debug_bounds("ipad-keyboard-row-tab-number-herdr-prefix-8")
        .unwrap();
    assert!(after_first.left() < first.left(), "the cell did not scroll");
    assert!(
        after_last.right() <= cell.right() + px(1.),
        "the last binding is unreachable"
    );
    assert_eq!(
        cx.debug_bounds("ipad-keyboard-row-tab-number-ipad")
            .unwrap(),
        other
    );
}
