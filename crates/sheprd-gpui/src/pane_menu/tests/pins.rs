use super::*;
use crate::{preferences::AgentSort, state::ConnectionStatus};

fn fixture(window: &mut Window, cx: &mut Context<SheprdWindow>) -> SheprdWindow {
    let mut view = crate::sidebar::layout_tests::fixture_window(window, cx);
    let mut listing = snapshot();
    listing.agent_view_label = None;
    let template = listing.agents[0].clone();
    listing.agents = listing
        .panes
        .iter()
        .map(|pane| sheprd_client::protocol::ClientShellAgent {
            pane_id: pane.pane_id.clone(),
            workspace_id: pane.workspace_id.clone(),
            tab_id: pane.tab_id.clone(),
            ..template.clone()
        })
        .collect();
    listing.agents[0].agent_status = sheprd_client::protocol::AgentStatus::Idle;
    view.live.snapshot = Some(Arc::new(listing));
    view.live.status = ConnectionStatus::Connected;
    view
}

#[gpui::test]
fn agent_menu_pin_unpin_repin_keeps_regular_order_and_rename(cx: &mut TestAppContext) {
    let (view, cx) = cx.add_window_view(fixture);
    cx.update(|window, cx| {
        view.update(cx, |view, cx| {
            for sort in [AgentSort::Grouped, AgentSort::Priority] {
                view.agent_sort = sort;
                view.agent_pins = Default::default();
                let regular: Vec<_> = view
                    .sidebar_agents()
                    .iter()
                    .map(|(host, pane)| (*host, pane.to_string()))
                    .collect();
                let a = regular[0].1.clone();
                let b = regular[1].1.clone();
                for pane in [&a, &b] {
                    view.open_agent_menu("local", pane, Point::default(), window, cx);
                    let menu = view.menu.pane.as_ref().unwrap();
                    assert!(
                        menu.actions()
                            .iter()
                            .any(|action| matches!(action, Action::Pin))
                    );
                    assert_eq!(Action::Pin.label(&menu.target), "Pin Agent");
                    view.activate_pane_menu(Action::Pin, window, cx);
                    assert!(view.menu.page.is_none());
                }
                for _ in 0..3 {
                    assert_eq!(view.sidebar_agents()[0].1, b);
                    assert_eq!(view.sidebar_agents()[1].1, a);
                    assert_eq!(view.sidebar_agents().len(), regular.len());
                }
                view.open_agent_menu("local", &a, Point::default(), window, cx);
                assert_eq!(
                    Action::Pin.label(&view.menu.pane.as_ref().unwrap().target),
                    "Unpin Agent"
                );
                view.activate_pane_menu(Action::Pin, window, cx);
                view.open_agent_menu("local", &a, Point::default(), window, cx);
                view.activate_pane_menu(Action::Pin, window, cx);
                assert_eq!(view.sidebar_agents()[0].1, a);
                // Rename changes presentation, never identity.
                let mut renamed = (**view.live.snapshot.as_ref().unwrap()).clone();
                renamed
                    .panes
                    .iter_mut()
                    .find(|pane| pane.pane_id == a)
                    .unwrap()
                    .label = Some("Renamed".into());
                view.live.snapshot = Some(Arc::new(renamed));
                assert!(view.agent_pin_rank(0, &a).is_some());
                for pane in [&a, &b] {
                    view.open_agent_menu("local", pane, Point::default(), window, cx);
                    view.activate_pane_menu(Action::Pin, window, cx);
                }
                let restored: Vec<_> = view
                    .sidebar_agents()
                    .iter()
                    .map(|(host, pane)| (*host, pane.to_string()))
                    .collect();
                assert_eq!(restored, regular);
            }
        })
    });
}

#[gpui::test]
fn two_hosts_equal_pane_ids_and_disconnects_do_not_share_pins(cx: &mut TestAppContext) {
    let (view, cx) = cx.add_window_view(fixture);
    view.update(cx, |view, _| {
        let mut remote = crate::endpoint::Endpoint::new(
            "ssh:test".into(),
            "Other".into(),
            sheprd_client::ConnectTarget::Ssh {
                target: "unused".into(),
                session: "default".into(),
            },
            true,
        );
        remote.live.snapshot = view.live.snapshot.clone();
        remote.live.status = ConnectionStatus::Connected;
        view.endpoints.push(remote);
        let snapshot = view.live.snapshot.as_ref().unwrap();
        let pane = snapshot.agents[0].pane_id.clone();
        view.agent_pins
            .toggle(crate::preferences::pins::AgentPin::new(
                "ssh:test",
                &snapshot.boot_id,
                &pane,
            ));
        assert!(view.agent_pin_rank(0, &pane).is_none());
        assert_eq!(view.sidebar_agents()[0], (1, pane.as_str()));
        view.endpoints[1].live.status = ConnectionStatus::Detached;
        view.reconcile_agent_pins();
        assert_eq!(view.sidebar_agents()[0], (1, pane.as_str()));
        let mut closed = (**view.endpoints[1].live.snapshot.as_ref().unwrap()).clone();
        closed.panes.retain(|p| p.pane_id != pane);
        view.endpoints[1].live.snapshot = Some(Arc::new(closed));
        view.endpoints[1].live.status = ConnectionStatus::Connected;
        view.reconcile_agent_pins();
        assert!(view.agent_pin_rank(1, &pane).is_none());
    });
}
