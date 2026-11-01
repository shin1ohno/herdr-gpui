use super::super::{Error, Hub, Remote, Start, State, panes::Panes};
use crate::{
    HerdrWindow,
    browser::{Location, Store, TabId},
    sidebar::layout_tests::fixture_window,
};
use gpui::Context;
use herdr_backend::TerminalId;
use herdr_client::ConnectTarget;
use std::{
    sync::{Mutex, mpsc},
    time::Duration,
};

static ATTEMPTS: Mutex<Option<mpsc::SyncSender<(ConnectTarget, Start)>>> = Mutex::new(None);

fn join(target: &ConnectTarget, start: Start) -> Result<herdr_core_client::CoreBackend, Error> {
    ATTEMPTS
        .lock()
        .unwrap()
        .as_ref()
        .unwrap()
        .send((target.clone(), start))
        .unwrap();
    Err(Error::Unavailable)
}

fn restore(view: &mut HerdrWindow, cx: &mut Context<HerdrWindow>) -> (TabId, ConnectTarget) {
    Store::default().install(cx);
    let snapshot = std::sync::Arc::make_mut(view.live.snapshot.as_mut().unwrap());
    snapshot.focused_workspace_id = snapshot
        .workspaces
        .first()
        .map(|workspace| workspace.workspace_id.clone());
    snapshot.focused_tab_id = snapshot.tabs.first().map(|tab| tab.tab_id.clone());
    view.core_terminals.hub = Hub::with_join(join);
    #[cfg(target_os = "ios")]
    {
        view.endpoints[0].id = "ssh:reconnect-test".into();
        view.endpoints[0].connection.target = ConnectTarget::Ssh {
            target: "user@box".into(),
            session: "work".into(),
        };
    }
    view.select_core_target(cx);
    #[cfg(target_os = "ios")]
    view.open_core_workspace();
    let target = view.core_terminals.hub.target().clone();
    let (scope, workspace) = view.browser_key().unwrap();
    let tab = Store::update(cx, |store| {
        store.open(
            scope,
            &workspace,
            Some(Location::CoreTerminal {
                panes: Panes::single(TerminalId::new("t1").unwrap()),
                observe: false,
                remote: Remote::from_target(&target),
            }),
            None,
        )
    })
    .unwrap();
    // A restored, detached tab can be laid out without any connection attempt.
    view.core_terminals.hub.detach();
    view.sync_core_views(cx);
    (tab, target)
}

#[gpui::test]
fn unavailable_core_tabs_reconnect_from_the_placeholder_and_sidebar(cx: &mut gpui::TestAppContext) {
    let (attempts, received) = mpsc::sync_channel(4);
    *ATTEMPTS.lock().unwrap() = Some(attempts);
    let (view, cx) = cx.add_window_view(fixture_window);
    let mut restored = None;
    view.update_in(cx, |view, window, cx| {
        let (tab, target) = restore(view, cx);
        view.show_browser_tab(tab, window, cx);
        assert!(matches!(view.core_terminals.hub.state(), State::Detached));
        restored = Some((tab, target));
    });
    cx.update(|window, cx| window.draw(cx).clear(cx));
    assert!(matches!(
        received.try_recv(),
        Err(mpsc::TryRecvError::Empty)
    ));
    let button = cx.debug_bounds("core-reconnect").unwrap();
    cx.simulate_click(button.center(), gpui::Modifiers::default());
    let (tab, target) = restored.unwrap();
    assert_eq!(
        received.recv_timeout(Duration::from_secs(5)).unwrap(),
        (target.clone(), Start::IfMissing)
    );
    view.read_with(cx, |view, _| {
        assert!(view.core_terminals.wanted.is_none());
        assert!(view.core_terminals.starting.is_none());
        assert_eq!(view.core_terminals.focus_wanted, Some(tab));
    });

    view.update(cx, |view, cx| {
        view.detach_core_connection(cx);
        view.sync_core_views(cx);
    });
    cx.update(|window, cx| window.draw(cx).clear(cx));
    let row = cx
        .debug_bounds(format!("core-sidebar-{}-t1", u64::from(tab)).leak())
        .unwrap();
    cx.simulate_click(row.center(), gpui::Modifiers::default());
    assert_eq!(
        received.recv_timeout(Duration::from_secs(5)).unwrap(),
        (target, Start::IfMissing)
    );
    view.read_with(cx, |view, cx| {
        assert!(view.core_terminals.wanted.is_none());
        assert!(view.core_terminals.starting.is_none());
        assert_eq!(
            cx.global::<Store>()
                .tabs()
                .map(|tab| tab.id)
                .collect::<Vec<_>>(),
            vec![tab]
        );
    });
    *ATTEMPTS.lock().unwrap() = None;
}

#[gpui::test]
fn desktop_core_placeholders_offer_no_reconnect(cx: &mut gpui::TestAppContext) {
    let _desktop = crate::ipad_ui::desktop();
    let (view, cx) = cx.add_window_view(fixture_window);
    view.update_in(cx, |view, window, cx| {
        let (tab, _) = restore(view, cx);
        view.show_browser_tab(tab, window, cx);
        let restored = cx.global::<Store>().get(tab).unwrap();
        assert!(!view.core_tab_can_reconnect(restored));
    });
    cx.update(|window, cx| window.draw(cx).clear(cx));
    assert!(cx.debug_bounds("core-reconnect").is_none());
}
