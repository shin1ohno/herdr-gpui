//! Presentation labels only; registered names and daemon identifiers stay intact.

use herdr_client::protocol::{ClientShellAgent, ClientShellSnapshot};

use super::agents::{PANE_CAPTION_LIMIT, safe_caption};

/// The chosen title-first policy puts a pane Rename before reported task metadata,
/// then a registered name, then custom presentation metadata. No OSC title is
/// treated as task metadata, and no identifier's spelling implies its origin.
pub(crate) fn agent_label(agent: &ClientShellAgent, snapshot: &ClientShellSnapshot) -> String {
    let pane = snapshot.panes.iter().find(|pane| {
        pane.pane_id == agent.pane_id
            && pane.tab_id == agent.tab_id
            && pane.workspace_id == agent.workspace_id
    });
    if let Some(label) = pane.and_then(|pane| safe_caption(pane.label.as_deref())) {
        return label;
    }
    for value in [&agent.title, &agent.name, &agent.display_agent] {
        if let Some(label) = safe_caption(value.as_deref())
            && !kind_only(&label, agent)
        {
            return label;
        }
    }
    // A location is honest when no task name was reported. Include the pane's
    // address even for a named tab: multiple agents can share that tab.
    let tab = snapshot
        .tabs
        .iter()
        .find(|tab| tab.tab_id == agent.tab_id && tab.workspace_id == agent.workspace_id)
        .and_then(|tab| safe_caption(Some(&tab.label)));
    let address = safe_caption(Some(&agent.pane_id)).unwrap_or_default();
    let location = tab.map_or_else(|| address.clone(), |tab| format!("{address} · {tab}"));
    herdr_client::text::safe_text(&location, PANE_CAPTION_LIMIT)
}

fn kind_only(label: &str, agent: &ClientShellAgent) -> bool {
    label.eq_ignore_ascii_case("agent")
        || crate::icons::AgentIcon::from_identity(Some(&label.to_ascii_lowercase()))
            != crate::icons::AgentIcon::Generic
        || agent
            .agent
            .as_deref()
            .is_some_and(|kind| label.eq_ignore_ascii_case(kind.trim()))
        // The daemon's canonical identity is `claude`; its product display
        // name carries no task information either. Match whole values only.
        || label.eq_ignore_ascii_case("Claude Code") // i18n: data
}
