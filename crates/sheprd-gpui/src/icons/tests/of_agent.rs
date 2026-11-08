#![allow(clippy::unwrap_used)]
use super::*;

fn agent(
    identity: Option<&str>,
    display: Option<&str>,
) -> sheprd_client::protocol::ClientShellAgent {
    let snapshot: sheprd_client::protocol::ClientShellSnapshot = serde_json::from_str(
        include_str!("../../../../sheprd-protocol/tests/fixtures/endpoint-snapshot-v1.json"),
    )
    .unwrap();
    let mut agent = snapshot.agents[0].clone();
    agent.agent = identity.map(str::to_owned);
    agent.display_agent = display.map(str::to_owned);
    agent
}

#[test]
fn ipad_generic_agents_fall_back_to_harness_display_identity() {
    let acp = agent(Some("acp"), Some("Claude"));
    assert_eq!(of_agent(&acp), AgentIcon::Generic);
    let _ipad = crate::ipad_ui::ipad_layout();
    assert_eq!(of_agent(&acp), AgentIcon::Claude);
    assert_eq!(of_agent(&agent(None, Some("CoDeX"))), AgentIcon::Codex);
    assert_eq!(
        of_agent(&agent(Some("future-agent"), Some("Gemini"))),
        AgentIcon::Gemini
    );
    assert_eq!(
        of_agent(&agent(Some("codex"), Some("Claude"))),
        AgentIcon::Codex
    );
    assert_eq!(of_agent(&agent(Some("acp"), None)), AgentIcon::Generic);
    assert_eq!(of_agent(&agent(None, Some("unknown"))), AgentIcon::Generic);
    let _desktop = crate::ipad_ui::desktop();
    assert_eq!(of_agent(&acp), AgentIcon::Generic);
}
