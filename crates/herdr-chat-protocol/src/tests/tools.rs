use super::*;

#[test]
fn tool_lookup_matches_call_ids_among_other_entries() {
    let mut transcript = Transcript::new("Claude");
    transcript.entries = vec![agent(2, "call-1"), tool(9, ToolStatus::Pending)];
    let found = transcript.tool("call-1").unwrap();
    assert_eq!(found.title, "cargo test");
    assert_eq!(found.kind, ToolKind::Execute);
    assert_eq!(found.status, ToolStatus::Pending);
    assert!(transcript.tool("call").is_none());
    assert!(transcript.tool("").is_none());
    assert!(transcript.tool("missing").is_none());
    transcript.entries.clear();
    assert!(transcript.tool("call-1").is_none());
}
