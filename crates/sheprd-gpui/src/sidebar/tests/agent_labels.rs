use super::*;
use crate::{LiveState, sidebar::agent_label};
use sheprd_client::ClientEvent;
use std::sync::Arc;

fn fixture() -> ClientShellSnapshot {
    let mut snapshot = layout_tests::snapshot(1);
    snapshot.agents.truncate(1);
    let agent = &mut snapshot.agents[0];
    agent.name = Some("registered-reviewer".into());
    agent.title = Some("Review keyboard input".into());
    snapshot.panes = vec![ClientShellPane {
        pane_id: agent.pane_id.clone(),
        workspace_id: agent.workspace_id.clone(),
        tab_id: agent.tab_id.clone(),
        label: None,
        cwd: Some("/not-the-right-column".into()),
        foreground_cwd: None,
        focused: false,
        right_click_passthrough: false,
    }];
    snapshot
}

fn label(snapshot: &ClientShellSnapshot) -> String {
    agent_label(&snapshot.agents[0], snapshot)
}

#[test]
fn titles_distinguish_same_kind_agents_without_changing_identity() {
    let mut snapshot = fixture();
    let original = snapshot.clone();
    let mut second = snapshot.agents[0].clone();
    second.pane_id = "p2".into();
    second.title = Some("Repair sidebar names".into());
    snapshot.agents.push(second);
    assert_eq!(label(&snapshot), "Review keyboard input");
    assert_eq!(
        agent_label(&snapshot.agents[1], &snapshot),
        "Repair sidebar names"
    );
    assert_eq!(snapshot.agents[0], original.agents[0]);
    assert_eq!(
        crate::icons::of_agent(&snapshot.agents[0]),
        crate::icons::of_agent(&snapshot.agents[1])
    );
}

#[test]
fn rename_wins_after_snapshot_refresh_and_clear_restores_title() {
    let mut snapshot = fixture();
    let mut live = LiveState::default();
    for (revision, rename, title, expected) in [
        (1, None, "Initial task", "Initial task"),
        (2, Some("My chosen name"), "Initial task", "My chosen name"),
        (3, Some("My chosen name"), "Updated task", "My chosen name"),
        (4, None, "Updated task", "Updated task"),
    ] {
        snapshot.revision = revision;
        snapshot.panes[0].label = rename.map(str::to_owned);
        snapshot.agents[0].title = Some(title.into());
        live.apply(ClientEvent::Snapshot(Arc::new(snapshot.clone())));
        assert_eq!(label(live.snapshot.as_deref().unwrap()), expected);
    }
}

#[test]
fn meaningful_names_and_custom_presentation_are_fallbacks_without_id_heuristics() {
    let mut snapshot = fixture();
    for name in [
        "codex migration reviewer",
        "cos-control-id",
        "claude-api cleanup",
    ] {
        snapshot.agents[0].name = Some(name.into());
        snapshot.agents[0].title = Some("claude".into());
        assert_eq!(label(&snapshot), name);
    }
    snapshot.agents[0].name = None;
    snapshot.agents[0].display_agent = Some("Team auth reviewer".into());
    assert_eq!(label(&snapshot), "Team auth reviewer");
    snapshot.agents[0].title = Some("Reported task".into());
    assert_eq!(label(&snapshot), "Reported task");
}

#[test]
fn missing_or_generic_titles_use_real_tab_and_pane_not_osc_or_cwd() {
    let mut snapshot = fixture();
    snapshot.agents[0].name = Some("claude".into());
    snapshot.agents[0].terminal_title = Some("Untrusted OSC task".into());
    snapshot.agents[0].terminal_title_stripped = Some("OSC task".into());
    for title in [
        None,
        Some(" \t "),
        Some("Claude Code"),
        Some("CLAUDE"),
        Some("agent"),
    ] {
        snapshot.agents[0].title = title.map(str::to_owned);
        assert_eq!(label(&snapshot), "p0 · tab 1");
    }
    snapshot.tabs.clear();
    assert_eq!(label(&snapshot), "p0");
    snapshot.panes[0].label = Some("claude".into());
    assert_eq!(label(&snapshot), "claude", "explicit GUI names always win");
}

#[test]
fn labels_are_bounded_safe_and_ignore_unrelated_panes() {
    let mut snapshot = fixture();
    snapshot.panes[0].label = Some("other pane".into());
    snapshot.panes[0].workspace_id = "another-workspace".into();
    assert_eq!(label(&snapshot), "Review keyboard input");
    snapshot.agents[0].title = Some("\u{202e}\u{7}actual title".into());
    assert_eq!(label(&snapshot), "actual title");
    snapshot.agents[0].title = Some("x".repeat(10_000));
    assert_eq!(label(&snapshot).chars().count(), PANE_CAPTION_LIMIT);
    snapshot.agents[0].title = Some("<pasted_content id=\"x\">Task</pasted_content>".into());
    assert_eq!(label(&snapshot), "Task");
}

#[test]
fn naming_does_not_change_order_or_ipad_metadata() {
    let mut snapshot = fixture();
    let before = snapshot.clone();
    snapshot.agents[0].title = Some("A new task".into());
    snapshot.panes[0].label = Some("Renamed".into());
    for sort in [
        crate::preferences::AgentSort::Grouped,
        crate::preferences::AgentSort::Priority,
    ] {
        let ids = |snapshot: &ClientShellSnapshot| {
            super::super::agents::sorted_agents(snapshot, sort)
                .iter()
                .map(|agent| agent.pane_id.clone())
                .collect::<Vec<_>>()
        };
        assert_eq!(ids(&before), ids(&snapshot));
    }
    let meta = super::super::cell::ipad_meta::agent_meta;
    assert_eq!(
        meta(&before.agents[0], &before),
        meta(&snapshot.agents[0], &snapshot)
    );
}

#[test]
fn accessibility_uses_the_renamed_primary_without_repeating_the_title() {
    let mut snapshot = fixture();
    snapshot.panes[0].label = Some("Chosen label".into());
    let agent = &snapshot.agents[0];
    let name = label(&snapshot);
    let pane = pane_name(agent, &snapshot, &name);
    assert_eq!(pane, None);
    let spoken =
        super::super::accessibility::agent_name(&name, pane.as_deref(), "working", None, None);
    assert!(spoken.contains("Chosen label"));
    assert!(!spoken.contains("Review keyboard input"));
}
