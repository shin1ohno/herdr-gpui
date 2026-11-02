use super::*;
use crate::sidebar::cell::ipad_meta::{agent_meta, lead_agent, needs_reason, space_meta};

#[test]
fn ipad_lead_prefers_focus_then_status_and_keeps_stable_ties() {
    let mut snapshot = layout_tests::snapshot(2);
    let mut second = snapshot.agents[0].clone();
    second.pane_id = "p2".into();
    second.agent_status = AgentStatus::Blocked;
    snapshot.agents.push(second.clone());
    assert_eq!(lead_agent(&snapshot, "w0").unwrap().pane_id, "p2");
    snapshot.agents[0].focused = true;
    assert_eq!(lead_agent(&snapshot, "w0").unwrap().pane_id, "p0");
    snapshot.agents[0].focused = false;
    snapshot.agents[0].agent_status = AgentStatus::Blocked;
    assert_eq!(lead_agent(&snapshot, "w0").unwrap().pane_id, "p0");
    assert_eq!(lead_agent(&snapshot, "missing"), None);
    // An urgent agent of a different workspace cannot become the lead.
    snapshot.agents[1].focused = true;
    assert_eq!(lead_agent(&snapshot, "w0").unwrap().pane_id, "p0");
}

#[test]
fn ipad_meta_counts_only_tabs_of_the_space_and_has_no_time_dependency() {
    let _ipad = crate::ipad_ui::ipad_layout();
    let mut snapshot = layout_tests::snapshot(2);
    let tab = snapshot.tabs[0].clone();
    snapshot.tabs = vec![
        tab.clone(),
        ClientShellTab {
            tab_id: "second".into(),
            ..tab.clone()
        },
        ClientShellTab {
            tab_id: "other".into(),
            workspace_id: "w1".into(),
            ..tab
        },
    ];
    let meta = space_meta(&snapshot, &snapshot.workspaces[0], |_| None);
    assert_eq!(meta.tab_count, 2);
    assert_eq!(meta.branch.as_deref(), Some("main"));
    assert!(!meta.needs_you);
    assert_eq!(meta.lead.unwrap().icon, crate::icons::AgentIcon::Claude);
    snapshot.agents.clear();
    snapshot.tabs.clear();
    let meta = space_meta(&snapshot, &snapshot.workspaces[0], |_| None);
    assert_eq!(meta.tab_count, 0);
    assert_eq!(meta.lead, None);
    assert_eq!(meta.reason, None);
}

use herdr_client::protocol::ClientShellTab;

#[test]
fn ipad_needs_you_prefers_permission_then_terminal_label_then_translated_fallback() {
    let mut agent = layout_tests::snapshot(1).agents.remove(0);
    agent.agent_status = AgentStatus::Blocked;
    agent.state_labels = vec![("blocked".into(), "  confirm the command  ".into())];
    assert_eq!(
        needs_reason(&agent, Some("Allow cargo test?")),
        "Allow cargo test?"
    );
    assert_eq!(needs_reason(&agent, None), "confirm the command");
    agent.agent = Some("acp".into());
    assert_eq!(
        needs_reason(&agent, None),
        crate::i18n::fl!("ipad-sidebar-needs-you")
    );
    agent.agent = Some("claude".into());
    agent.state_labels.clear();
    assert_eq!(
        needs_reason(&agent, Some(" \n ")),
        crate::i18n::fl!("ipad-sidebar-needs-you")
    );
    assert_eq!(
        needs_reason(&agent, None),
        crate::i18n::fl!("ipad-sidebar-needs-you")
    );
    let unsafe_title = format!("\u{202e}\u{0007}{}", "x".repeat(500));
    let reason = needs_reason(&agent, Some(&unsafe_title));
    assert!(!reason.contains(['\u{202e}', '\u{0007}']));
    assert!(reason.chars().count() <= 161);
}

#[test]
fn ipad_space_reason_uses_a_blocked_member_even_when_the_focused_lead_is_working() {
    let mut snapshot = layout_tests::snapshot(1);
    snapshot.agents[0].focused = true;
    let mut blocked = snapshot.agents[0].clone();
    blocked.pane_id = "blocked".into();
    blocked.focused = false;
    blocked.agent_status = AgentStatus::Blocked;
    snapshot.agents.push(blocked);
    let meta = space_meta(&snapshot, &snapshot.workspaces[0], |agent| {
        assert_eq!(agent.pane_id, "blocked");
        Some("Approve the plan".into())
    });
    assert!(meta.needs_you);
    assert_eq!(meta.reason.as_deref(), Some("Approve the plan"));
    assert_eq!(
        meta.lead.unwrap().state,
        crate::sidebar::agents::status_text(AgentStatus::Working)
    );
}

#[test]
fn ipad_agent_meta_is_attention_done_branch_or_space() {
    let mut snapshot = layout_tests::snapshot(6);
    let mut agent = snapshot.agents[0].clone();
    assert_eq!(agent_meta(&agent, &snapshot).text, "herdr");
    agent.workspace_id = "w4".into();
    assert_eq!(agent_meta(&agent, &snapshot).text, "worktree/sidebar-child");
    agent.agent_status = AgentStatus::Done;
    assert_eq!(
        agent_meta(&agent, &snapshot).text,
        crate::sidebar::agents::status_text(AgentStatus::Done)
    );
    agent.agent_status = AgentStatus::Blocked;
    let meta = agent_meta(&agent, &snapshot);
    assert!(meta.needs_you);
    assert_eq!(meta.text, crate::i18n::fl!("ipad-sidebar-needs-you"));
    agent.agent_status = AgentStatus::Idle;
    snapshot.workspaces.clear();
    assert_eq!(agent_meta(&agent, &snapshot).text, "");
}

#[gpui::test]
fn ipad_chat_reason_reads_first_permission_only_from_the_selected_matching_endpoint(
    cx: &mut gpui::TestAppContext,
) {
    use herdr_chat_protocol::{
        Cursor, EndpointId, Permission, PermissionId, ServerMessage, Transcript,
    };
    let (view, cx) = cx.add_window_view(layout_tests::fixture_window);
    view.update(cx, |view, _| {
        let endpoint: EndpointId = "12345678123442348234123456789abc".parse().unwrap();
        let mut snapshot = layout_tests::snapshot(1);
        snapshot.agents.truncate(1);
        snapshot.agents[0].agent_status = AgentStatus::Blocked;
        snapshot.agents[0].agent = Some("acp".into());
        snapshot.agents[0].tokens = vec![(herdr_chat_protocol::TOKEN.into(), endpoint.to_string())];
        view.chats.reconcile(
            vec![crate::chat::Target {
                endpoint,
                existing: None,
                pane_id: "p0".into(),
                route: crate::chat::Route::Local,
            }],
            crate::motion::now(),
            &mut |_, _, _| Err(crate::chat::Error::NotAttached),
        );
        let pane = view.chats.pane_mut("p0").unwrap();
        pane.session
            .receive(ServerMessage::Welcome {
                agent: "Claude".into(),
                pane: Some("p0".into()),
                version: herdr_chat_protocol::VERSION,
            })
            .unwrap();
        let mut transcript = Transcript::new("Claude");
        transcript.permissions = ["Allow the command", "A later permission"]
            .into_iter()
            .enumerate()
            .map(|(i, title)| Permission {
                call_id: None,
                id: PermissionId(i as u64),
                options: vec![],
                title: title.into(),
            })
            .collect();
        pane.session
            .receive(ServerMessage::Snapshot {
                cursor: Cursor::new_epoch(),
                transcript: Box::new(transcript),
            })
            .unwrap();
        assert_eq!(
            view.ipad_space_meta(true, &snapshot, &snapshot.workspaces[0])
                .reason
                .as_deref(),
            Some("Allow the command")
        );
        assert_eq!(
            view.ipad_space_meta(false, &snapshot, &snapshot.workspaces[0])
                .reason,
            Some(crate::i18n::fl!("ipad-sidebar-needs-you"))
        );
        snapshot.agents[0].tokens[0].1 = "87654321123442348234123456789abc".into();
        assert_eq!(
            view.ipad_space_meta(true, &snapshot, &snapshot.workspaces[0])
                .reason,
            Some(crate::i18n::fl!("ipad-sidebar-needs-you"))
        );
    });
}
