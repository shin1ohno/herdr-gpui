//! iPad's sessions: the picker unfolds a saved Herdr 1 host over SSH, never
//! the local sentinel, and only the unfolded device's sessions are listed.
use super::*;
use crate::sidebar::layout_tests::fixture_window;

fn remote(id: &str) -> Endpoint {
    Endpoint::new(
        id.into(),
        id.into(),
        ConnectTarget::Ssh {
            target: "user@unused.invalid".into(),
            session: "default".into(),
        },
        true,
    )
}

#[gpui::test]
fn ipad_sessions_show_only_the_unfolded_remote_device(cx: &mut TestAppContext) {
    let (view, cx) = cx.add_window_view(fixture_window);
    view.update(cx, |view, _| {
        view.endpoints[0].id = crate::endpoint::LOCAL.into();
        for id in ["ssh:selected", "ssh:other"] {
            view.endpoints.push(remote(id));
        }
        view.selected_endpoint = 1;
        view.menu.page = Some(Page::Devices);
        view.unfold_device(1);
        assert_eq!(view.menu.unfolded_device.as_deref(), Some("ssh:selected"));
        let lines = view.picker_lines();
        let rows: Vec<&Row> = lines
            .iter()
            .filter_map(|line| match line {
                Line::Session(entry) => entry.row(),
                _ => None,
            })
            .collect();
        assert!(rows.iter().any(|row| matches!(row,
            Row::Device { id, .. } if id == "ssh:selected"
        )));
        assert!(!rows.iter().any(|row| matches!(row,
            Row::Device { id, .. } if id == "ssh:other"
        )));
        assert!(!rows.iter().any(|row| matches!(row, Row::Local(_))));
        // The local sentinel is no device on iPad, so it has no line to unfold.
        let local = lines.iter().any(|line| matches!(line, Line::Device(0)));
        assert_eq!(local, !cfg!(target_os = "ios"));
    });
}

#[gpui::test]
fn opening_sessions_on_ipad_cannot_open_a_local_session_list(cx: &mut TestAppContext) {
    let (view, cx) = cx.add_window_view(fixture_window);
    view.update_in(cx, |view, window, cx| {
        view.open_sessions(point(px(10.), px(500.)), window, cx);
        assert_eq!(view.menu.page.is_some(), !cfg!(target_os = "ios"));
        view.dismiss_menu(window, cx);

        view.endpoints.push(remote("ssh:remote"));
        view.selected_endpoint = view.endpoints.len() - 1;
        view.open_sessions(point(px(10.), px(500.)), window, cx);
        assert_eq!(view.menu.page, Some(Page::Devices));
        assert_eq!(view.menu.unfolded_device.as_deref(), Some("ssh:remote"));
    });
}
