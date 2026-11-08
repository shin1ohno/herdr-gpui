//! Pure sidebar projection; no clocks, I/O, or client-local activity ledger.

use crate::{
    SheprdWindow,
    i18n::fl,
    icons::AgentIcon,
    sidebar::{
        agent_label,
        agents::{state_label, status_priority, status_text},
    },
};
use sheprd_client::protocol::{
    AgentStatus, ClientShellAgent, ClientShellSnapshot, ClientShellWorkspace,
};
use std::borrow::Cow;

#[derive(Debug, PartialEq, Eq)]
pub(in crate::sidebar) struct Lead {
    pub(in crate::sidebar) icon: AgentIcon,
    pub(in crate::sidebar) name: String,
    pub(in crate::sidebar) state: String,
}

#[derive(Debug, PartialEq, Eq)]
pub(in crate::sidebar) struct SpaceMeta {
    pub(in crate::sidebar) tab_count: usize,
    pub(in crate::sidebar) needs_you: bool,
    pub(in crate::sidebar) lead: Option<Lead>,
    pub(in crate::sidebar) branch: Option<String>,
    pub(in crate::sidebar) worktree: bool,
    pub(in crate::sidebar) reason: Option<String>,
}

#[derive(Debug, PartialEq, Eq)]
pub(in crate::sidebar) struct AgentMeta {
    pub(in crate::sidebar) text: String,
    pub(in crate::sidebar) needs_you: bool,
}

/// Focus wins; equal-priority agents retain the daemon's stable order.
pub(in crate::sidebar) fn lead_agent<'a>(
    snapshot: &'a ClientShellSnapshot,
    workspace: &str,
) -> Option<&'a ClientShellAgent> {
    snapshot
        .agents
        .iter()
        .filter(|agent| agent.workspace_id == workspace)
        .fold(None, |lead: Option<&ClientShellAgent>, agent| match lead {
            Some(lead)
                if (lead.focused, status_priority(lead.agent_status))
                    >= (agent.focused, status_priority(agent.agent_status)) =>
            {
                Some(lead)
            }
            _ => Some(agent),
        })
}

fn text(value: &str) -> Option<String> {
    let text = crate::notifications::safe_text(value, 160);
    let text = text.trim();
    (!text.is_empty()).then(|| text.to_owned())
}

pub(in crate::sidebar) fn needs_reason(
    agent: &ClientShellAgent,
    permission: Option<&str>,
) -> String {
    if let Some(title) = permission.and_then(text) {
        return title;
    }
    let chat = agent.agent.as_deref() == Some("acp")
        || agent
            .tokens
            .iter()
            .any(|(key, _)| key == sheprd_chat_protocol::TOKEN);
    if !chat && let Cow::Owned(label) = state_label(agent, "blocked") {
        return label;
    }
    fl!("ipad-sidebar-needs-you")
}

pub(in crate::sidebar) fn space_meta(
    snapshot: &ClientShellSnapshot,
    workspace: &ClientShellWorkspace,
    permission: impl FnOnce(&ClientShellAgent) -> Option<String>,
) -> SpaceMeta {
    let blocked = snapshot.agents.iter().find(|agent| {
        agent.workspace_id == workspace.workspace_id && agent.agent_status == AgentStatus::Blocked
    });
    let needs_you = blocked.is_some() || workspace.agent_status == AgentStatus::Blocked;
    let reason = needs_you.then(|| {
        blocked.map_or_else(
            || fl!("ipad-sidebar-needs-you"),
            |agent| needs_reason(agent, permission(agent).as_deref()),
        )
    });
    SpaceMeta {
        tab_count: snapshot
            .tabs
            .iter()
            .filter(|tab| tab.workspace_id == workspace.workspace_id)
            .count(),
        needs_you,
        lead: lead_agent(snapshot, &workspace.workspace_id).map(|agent| Lead {
            icon: crate::icons::of_agent(agent),
            name: agent_label(agent, snapshot),
            state: state_label(agent, &status_text(agent.agent_status)).into_owned(),
        }),
        branch: workspace.branch.as_deref().and_then(text),
        worktree: workspace
            .worktree
            .as_ref()
            .is_some_and(|tree| tree.is_linked_worktree),
        reason,
    }
}

pub(in crate::sidebar) fn agent_meta(
    agent: &ClientShellAgent,
    snapshot: &ClientShellSnapshot,
) -> AgentMeta {
    let needs_you = agent.agent_status == AgentStatus::Blocked;
    let workspace = snapshot
        .workspaces
        .iter()
        .find(|space| space.workspace_id == agent.workspace_id);
    let text = if needs_you {
        fl!("ipad-sidebar-needs-you")
    } else if agent.agent_status == AgentStatus::Done {
        status_text(AgentStatus::Done)
    } else if let Some(workspace) = workspace {
        let branch = workspace.branch.as_deref().and_then(text);
        if workspace
            .worktree
            .as_ref()
            .is_some_and(|tree| tree.is_linked_worktree)
        {
            branch
                .or_else(|| text(&workspace.label))
                .unwrap_or_default()
        } else {
            text(&workspace.label).or(branch).unwrap_or_default()
        }
    } else {
        String::new()
    };
    AgentMeta { text, needs_you }
}

impl SheprdWindow {
    /// Chats belong to the selected daemon. Other hosts must never borrow a
    /// same-named pane's permission. The ACP endpoint token fences reconnects.
    pub(in crate::sidebar) fn ipad_space_meta(
        &self,
        selected: bool,
        snapshot: &ClientShellSnapshot,
        workspace: &ClientShellWorkspace,
    ) -> SpaceMeta {
        space_meta(snapshot, workspace, |agent| {
            if !selected {
                return None;
            }
            let pane = self.chats.pane(&agent.pane_id)?;
            let endpoint: sheprd_chat_protocol::EndpointId = agent
                .tokens
                .iter()
                .find(|(key, _)| key == sheprd_chat_protocol::TOKEN)?
                .1
                .parse()
                .ok()?;
            if pane.target.endpoint != endpoint {
                return None;
            }
            pane.session
                .transcript()
                .permissions
                .first()
                .map(|permission| permission.title.clone())
        })
    }
}
