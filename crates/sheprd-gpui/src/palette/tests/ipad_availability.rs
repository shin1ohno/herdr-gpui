use super::*;

#[gpui::test]
fn ipad_excludes_unsupported_commands_and_desktop_keeps_them(cx: &mut TestAppContext) {
    let excluded = [
        // keep-sorted start
        Command::GitMenu,
        Command::GoToFile,
        Command::GoToSymbol,
        Command::InstallBrowserSkill,
        Command::NewBrowserTab,
        Command::OpenCode,
        Command::OpenOrchestrator,
        Command::OpenPx0,
        Command::RemotePair,
        Command::ToggleAnnotate,
        Command::ToggleFullScreen,
        // keep-sorted end
    ];
    let (view, cx) = cx.add_window_view(fixture_window);
    for ipad in [false, true] {
        let _ipad = ipad.then(crate::ipad_ui::ipad_layout);
        view.update_in(cx, |view, window, cx| {
            view.live.supports_pane_clear = true;
            view.live.supports_edit_scrollback = true;
            view.open_palette(Filter::Commands, window, cx);
            let palette = view.menu.palette.as_ref().unwrap();
            let offered: Vec<_> = palette
                .entries
                .iter()
                .filter_map(|entry| match entry.action {
                    Action::Native(command) => Some(command),
                    _ => None,
                })
                .collect();
            for command in excluded {
                assert_eq!(offered.contains(&command), !ipad, "{command:?}");
            }
            for info in COMMANDS {
                if matches!(info.command, Command::Palette | Command::ResumePalette)
                    || (info.command == Command::Quit && !crate::actions::QUIT_AVAILABLE)
                {
                    continue;
                }
                assert_eq!(
                    offered.contains(&info.command),
                    !ipad || !excluded.contains(&info.command),
                    "{:?}",
                    info.command
                );
            }
            view.dismiss_menu(window, cx);
        });
    }
}
