//! Group filtering keeps the coordinator reachable while preserving pin order.
use super::*;
use crate::preferences::pins::AgentPin;

#[gpui::test]
fn group_coordinator_leads_pinned_workers_and_all_agents_keep_pin_priority(
    cx: &mut gpui::TestAppContext,
) {
    let (view, cx) = cx.add_window_view(agent_groups::grouped);
    view.update(cx, |view, _| {
        let boot = &view.live.snapshot.as_ref().unwrap().boot_id;
        view.agent_pins.toggle(AgentPin::new("local", boot, "p0"));
        view.agent_pins.toggle(AgentPin::new("local", boot, "p1"));
    });
    let rows = |view: &SheprdWindow| {
        view.sidebar_agents()
            .into_iter()
            .map(|(_, pane)| pane.to_owned())
            .collect::<Vec<_>>()
    };
    view.read_with(cx, |view, _| assert_eq!(rows(view), ["p1", "p0", "p2"]));
    view.update(cx, |view, _| view.agent_group = Some("demo".into()));
    view.read_with(cx, |view, _| {
        assert_eq!(rows(view), ["p2", "p0"]);
        assert!(view.agent_pin_rank(0, "p0").is_some());
    });
    view.update(cx, |view, _| view.agent_group = None);
    view.read_with(cx, |view, _| assert_eq!(rows(view), ["p1", "p0", "p2"]));
}
