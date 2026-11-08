use super::*;

#[gpui::test]
fn compact_agent_pin_click_keeps_existing_actions_and_unpins(cx: &mut TestAppContext) {
    let _ipad = crate::ipad_ui::ipad_layout();
    let (view, cx) = fixture(cx);
    view.update(cx, |view, _| {
        view.live.status = crate::state::ConnectionStatus::Connected
    });
    for pinned in [true, false, true] {
        open(&view, cx, Some(NavigationTarget::Pane("p0")));
        for selector in [
            "ipad-rail-row-menu-Open",
            "ipad-rail-row-menu-NewTab",
            "ipad-rail-row-menu-OpenChat",
        ] {
            assert!(cx.debug_bounds(selector).is_some());
        }
        click(cx, "ipad-rail-row-menu-Pin");
        view.read_with(cx, |view, _| {
            assert_eq!(view.agent_pin_rank(0, "p0").is_some(), pinned);
            assert!(view.menu.page.is_none());
        });
    }
    open(&view, cx, Some(NavigationTarget::Workspace("w0")));
    assert!(cx.debug_bounds("ipad-rail-row-menu-Pin").is_none());
}
