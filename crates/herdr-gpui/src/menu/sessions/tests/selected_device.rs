use super::*;
use crate::sidebar::layout_tests::fixture_window;

#[gpui::test]
fn ipad_sessions_show_only_the_selected_remote_device(cx: &mut TestAppContext) {
    let (view, cx) = cx.add_window_view(fixture_window);
    view.update(cx, |view, _| {
        for id in ["ssh:selected", "ssh:other"] {
            view.endpoints.push(Endpoint::new(
                id.into(),
                id.into(),
                ConnectTarget::Ssh {
                    target: "user@unused.invalid".into(),
                    session: "default".into(),
                },
                true,
            ));
        }
        view.selected_endpoint = 1;
        let entries = view.session_entries();
        assert!(entries.iter().any(|entry| matches!(entry,
            Entry::Row { row: Row::Device { id, .. }, .. } if id == "ssh:selected"
        )));
        let has_local = entries.iter().any(|entry| {
            matches!(entry,
                Entry::Header { selector, .. } if selector == "sessions-local-header"
            )
        });
        assert_eq!(has_local, !cfg!(target_os = "ios"));
        let has_other = entries.iter().any(|entry| {
            matches!(entry,
                Entry::Row { row: Row::Device { id, .. }, .. } if id == "ssh:other"
            )
        });
        assert_eq!(has_other, !cfg!(target_os = "ios"));
    });
}

#[gpui::test]
fn opening_sessions_on_ipad_cannot_open_a_local_session_list(cx: &mut TestAppContext) {
    let (view, cx) = cx.add_window_view(fixture_window);
    view.update_in(cx, |view, window, cx| {
        view.open_sessions(point(px(10.), px(500.)), window, cx);
        assert_eq!(view.menu.page.is_some(), !cfg!(target_os = "ios"));
    });
}
