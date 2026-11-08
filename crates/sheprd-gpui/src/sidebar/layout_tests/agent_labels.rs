use super::*;

#[gpui::test]
fn ipad_primary_names_refresh_in_panel_and_rail(cx: &mut gpui::TestAppContext) {
    let _ipad = crate::ipad_ui::ipad_layout();
    let (view, cx) = cx.add_window_view(|window, cx| {
        let mut view = fixture_window(window, cx);
        let mut snapshot = snapshot(1);
        for (agent, title) in snapshot.agents.iter_mut().zip(["Review API", "Review UI"]) {
            agent.workspace_id = "w0".into();
            agent.tab_id = "t0".into();
            agent.title = Some(title.into());
        }
        snapshot.panes = snapshot
            .agents
            .iter()
            .map(|agent| ClientShellPane {
                pane_id: agent.pane_id.clone(),
                workspace_id: agent.workspace_id.clone(),
                tab_id: agent.tab_id.clone(),
                label: None,
                cwd: Some("/never-the-right-column".into()),
                foreground_cwd: None,
                focused: false,
                right_click_passthrough: false,
            })
            .collect();
        view.live.surface = None;
        view.live.snapshot = Some(Arc::new(snapshot));
        view.config.show_agents = true;
        view.config.agents_placement = crate::config::AgentsPlacement::Panel;
        view.sidebar_width = Some(480.);
        view
    });
    cx.simulate_resize(size(px(1194.), px(834.)));
    cx.run_until_parked();
    for (revision, rename, expected) in [
        (1, None, "Review API"),
        (2, Some("Chosen title"), "Chosen title"),
        (3, Some("Chosen title"), "Chosen title"),
        (4, None, "Review API"),
    ] {
        view.update(cx, |view, cx| {
            let mut snapshot = view.live.snapshot.as_deref().unwrap().clone();
            snapshot.revision = revision;
            snapshot.panes[0].label = rename.map(str::to_owned);
            view.live
                .apply(sheprd_client::ClientEvent::Snapshot(Arc::new(snapshot)));
            cx.notify();
        });
        cx.update(|window, cx| {
            cx.default_global::<TextProbes>().0.clear();
            full_draw(window, cx).clear(cx);
            let probes = &cx.global::<TextProbes>().0;
            assert!(probes.contains_key(expected), "{expected}: {probes:?}");
            assert!(probes.contains_key("Review UI"));
            assert!(!probes.contains_key("/never-the-right-column"));
        });
        for id in ["ipad-agent-name-agent-p0", "ipad-agent-meta-agent-p0"] {
            assert!(cx.debug_bounds(id).is_some(), "missing {id}");
        }
        view.update(cx, |view, cx| {
            view.sidebar_visible = false;
            cx.notify();
        });
        cx.update(|window, cx| {
            cx.default_global::<TextProbes>().0.clear();
            full_draw(window, cx).clear(cx);
            assert!(cx.global::<TextProbes>().0.contains_key(expected));
        });
        assert!(cx.debug_bounds("rail-agent-icon-local-p0").is_some());
        view.update(cx, |view, cx| {
            view.sidebar_visible = true;
            cx.notify();
        });
    }
}
