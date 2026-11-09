use super::*;

fn contains_action(menus: &[Menu], expected: &dyn gpui::Action) -> bool {
    menus.iter().any(|menu| {
        menu.items.iter().any(|item| match item {
            MenuItem::Action { action, .. } => action.partial_eq(expected),
            MenuItem::Submenu(menu) => contains_action(std::slice::from_ref(menu), expected),
            _ => false,
        })
    })
}

fn qa_config() -> Config {
    let mut config = Config::default();
    config.features.qa_menu = true;
    config
}

#[test]
fn ipad_menu_omits_unavailable_actions() {
    let model = menus_for_platform(&qa_config(), true);
    for expected in [
        Box::new(Quit) as Box<dyn gpui::Action>,
        Box::new(CheckForUpdates),
        Box::new(crate::actions::ShowSystemNotificationPreview),
    ] {
        assert!(!contains_action(&model, expected.as_ref()));
    }
    for command in [
        Command::ToggleFullScreen,
        Command::OpenCode,
        Command::OpenPx0,
        Command::GoToSymbol,
        Command::GoToFile,
    ] {
        assert!(!contains_action(&model, &RunCommand { command }));
    }
    #[cfg(target_os = "macos")]
    for expected in [
        Box::new(Hide) as Box<dyn gpui::Action>,
        Box::new(HideOthers),
        Box::new(ShowAll),
        Box::new(Minimize),
    ] {
        assert!(!contains_action(&model, expected.as_ref()));
    }
}

#[test]
fn ipad_menu_keeps_working_window_terminal_and_preview_actions() {
    let model = menus_for_platform(&qa_config(), true);
    for command in [
        Command::About,
        Command::Palette,
        Command::Settings,
        Command::Keybinds,
        Command::Workspace,
        Command::Tab,
        Command::SplitRight,
        Command::SplitDown,
        Command::EditScrollback,
        Command::Reconnect,
        Command::DevicesOverview,
        Command::ToggleZenMode,
        Command::NewWindow,
        Command::CycleWindows,
        Command::CloseWindow,
    ] {
        assert!(contains_action(&model, &RunCommand { command }));
    }
    for expected in [
        Box::new(ShowLogs) as Box<dyn gpui::Action>,
        Box::new(ShowUpdatePreview),
        Box::new(ShowUpdateDownloadPreview),
        Box::new(ShowUpdateHomebrewPreview),
        Box::new(PlaySound),
        Box::new(RingBellPreview),
        Box::new(ToggleSlowAnimations),
    ] {
        assert!(contains_action(&model, expected.as_ref()));
    }
    let edit = model
        .iter()
        .find(|menu| menu.name.as_ref() == "Edit")
        .unwrap();
    let native_actions: Vec<_> = edit
        .items
        .iter()
        .filter_map(|item| match item {
            MenuItem::Action { os_action, .. } => *os_action,
            _ => None,
        })
        .collect();
    assert!(
        native_actions
            == [
                OsAction::Cut,
                OsAction::Copy,
                OsAction::Paste,
                OsAction::SelectAll
            ]
    );
}

#[test]
fn ipad_menu_has_no_empty_separator_groups() {
    fn check(model: &[Menu]) {
        for menu in model {
            assert!(!menu.items.is_empty(), "{}", menu.name);
            assert!(!matches!(menu.items.first(), Some(MenuItem::Separator)));
            assert!(!matches!(menu.items.last(), Some(MenuItem::Separator)));
            for pair in menu.items.windows(2) {
                assert!(!matches!(pair, [MenuItem::Separator, MenuItem::Separator]));
            }
            for item in &menu.items {
                if let MenuItem::Submenu(menu) = item {
                    check(std::slice::from_ref(menu));
                }
            }
        }
    }
    check(&menus_for_platform(&qa_config(), true));
}

#[test]
fn desktop_menu_keeps_desktop_actions() {
    let model = menus_for_platform(&qa_config(), false);
    assert_eq!(
        contains_action(&model, &Quit),
        crate::actions::QUIT_AVAILABLE
    );
    assert!(contains_action(&model, &CheckForUpdates));
    assert!(contains_action(
        &model,
        &crate::actions::ShowSystemNotificationPreview
    ));
    for command in [
        Command::ToggleFullScreen,
        Command::OpenCode,
        Command::OpenPx0,
        Command::GoToSymbol,
        Command::GoToFile,
    ] {
        assert!(contains_action(&model, &RunCommand { command }));
    }
}

#[test]
fn ipad_menu_names_are_localized_under_en_xa() {
    crate::i18n::leak_tests::under_en_xa("iPad menu bar en-XA", |_, _| {
        fn check(model: &[Menu]) {
            for menu in model {
                assert!(sheprd_i18n::pseudo::is_marked(&menu.name), "{}", menu.name);
                for item in &menu.items {
                    match item {
                        MenuItem::Action { name, .. } => {
                            assert!(sheprd_i18n::pseudo::is_marked(name), "{name}");
                        }
                        MenuItem::Submenu(menu) => check(std::slice::from_ref(menu)),
                        _ => {}
                    }
                }
            }
        }
        check(&menus_for_platform(&qa_config(), true));
    });
}
