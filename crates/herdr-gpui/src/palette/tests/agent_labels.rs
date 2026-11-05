use super::*;

#[test]
fn title_first_labels_still_find_registered_names_and_custom_display_metadata() {
    let mut snapshot = go_to_fixture();
    snapshot.panes[0].label = None;
    snapshot.agents[0].title = Some("Repair key routing".into());
    snapshot.agents[0].name = Some("cos-stable-control-name".into());
    snapshot.agents[0].display_agent = Some("Keyboard reviewer".into());
    let mut entries = Vec::new();
    go_to_entries(crate::endpoint::LOCAL, None, &snapshot, &mut entries);
    let target = Identity::Go(
        crate::endpoint::LOCAL.into(),
        NavigationTarget::Pane("w1:p1".into()),
    );
    let entry = entries
        .iter()
        .find(|entry| entry.action.identity() == target)
        .unwrap();
    assert_eq!(entry.label.as_ref(), "Repair key routing");
    for query in [
        "Repair key routing",
        "cos-stable-control-name",
        "Keyboard reviewer",
        "claude",
    ] {
        let hits = search::rank(
            &entries,
            0..entries.len(),
            &search::Query::parse(query),
            &mut search::matcher(),
        );
        assert!(
            hits.iter()
                .any(|hit| entries[hit.index].action.identity() == target),
            "{query}"
        );
    }
}
