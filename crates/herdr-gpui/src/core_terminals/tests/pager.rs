use crate::{
    HerdrWindow,
    browser::{Location, Pick, Store, TabId},
    controls::Command,
    core_terminals::{
        TabPanes,
        panes::{Axis, Panes},
        view::CoreTerminalView,
        worker::Link,
    },
    ipad_keys::Page,
    sidebar::layout_tests::fixture_window,
};
use core::prelude::v1::test;
use gpui::*;
use herdr_backend::TerminalId;

fn install(view: &mut HerdrWindow, window: &mut Window, cx: &mut Context<HerdrWindow>) -> TabId {
    view.endpoints[0].connection.target = herdr_client::ConnectTarget::Local;
    std::sync::Arc::make_mut(view.live.snapshot.as_mut().unwrap()).focused_workspace_id =
        Some("w1".into());
    let scope = crate::browser::scope(&view.endpoints[0]);
    view.core_terminals.scope = Some(scope.clone());
    let first = TerminalId::new("pager-first").unwrap();
    let second = TerminalId::new("pager-second").unwrap();
    let mut panes = Panes::single(first.clone());
    panes.split(&first, Axis::Row, second.clone()).unwrap();
    let tab = Store::update(cx, |store| {
        store.open(
            scope,
            "w1",
            Some(Location::CoreTerminal {
                panes: panes.clone(),
                observe: false,
                remote: None,
            }),
            None,
        )
    })
    .unwrap();
    view.core_terminals
        .tabs
        .insert(tab, TabPanes::new(panes, false));
    for terminal in [first, second] {
        let (link, _) = Link::recording();
        let look = view.core_look();
        let painter = view.painter.clone();
        let core =
            cx.new(|cx| CoreTerminalView::new(terminal.clone(), link, look, painter, false, cx));
        view.core_terminals.views.insert((tab, terminal), core);
    }
    let layout = view.ensure_layout().unwrap();
    let group = layout.active();
    layout.show(group, Pick::Page(tab));
    view.focus_core_terminal(tab, window, cx);
    tab
}

#[gpui::test]
fn pager_next_and_previous_keep_zoom_and_follow_the_active_pane(cx: &mut TestAppContext) {
    let _ipad = crate::ipad_ui::ipad_layout();
    let (view, cx) = cx.add_window_view(fixture_window);
    view.update_in(cx, |view, window, cx| {
        let tab = install(view, window, cx);
        view.ipad_core_single_pane(true, cx);
        assert!(view.core_terminals.tabs[&tab].zoom.is_open());
        assert!(view.core_pane_command(Command::NextPane, window, cx));
        let shown = &view.core_terminals.tabs[&tab];
        assert_eq!(shown.active.as_str(), "pager-second");
        assert_eq!(shown.zoomed.as_ref(), Some(&shown.active));
        assert!(shown.zoom.is_open());
        view.ipad_core_page(tab, Page::Next, window, cx);
        assert_eq!(
            view.core_terminals.tabs[&tab].active.as_str(),
            "pager-first"
        );
        view.ipad_core_page(tab, Page::Previous, window, cx);
        assert_eq!(
            view.core_terminals.tabs[&tab].active.as_str(),
            "pager-second"
        );
        view.ipad_core_single_pane(false, cx);
        assert!(!view.core_terminals.tabs[&tab].zoom.is_open());
        assert!(view.ipad.pane_zoom.is_empty());
    });
}

#[gpui::test]
fn restoring_zoom_does_not_hide_the_selected_pane_and_stale_dots_do_nothing(
    cx: &mut TestAppContext,
) {
    let (view, cx) = cx.add_window_view(fixture_window);
    view.update_in(cx, |view, window, cx| {
        let tab = install(view, window, cx);
        let original = view.core_terminals.tabs[&tab].active.clone();
        view.core_terminals
            .tabs
            .get_mut(&tab)
            .unwrap()
            .set_zoom(Some(original.clone()));
        view.ipad_core_single_pane(true, cx);
        view.ipad_core_page(tab, Page::At(1), window, cx);
        view.ipad_core_page(tab, Page::At(100), window, cx);
        assert_eq!(
            view.core_terminals.tabs[&tab].active.as_str(),
            "pager-second"
        );
        view.ipad_core_single_pane(false, cx);
        let shown = &view.core_terminals.tabs[&tab];
        assert_eq!(shown.zoomed.as_ref(), Some(&shown.active));
        assert!(view.core_terminals.tabs[&tab].zoom.is_open());
    });
}

#[gpui::test]
fn ordinary_directional_focus_still_unzooms_on_desktop(cx: &mut TestAppContext) {
    let _desktop = crate::ipad_ui::desktop();
    let (view, cx) = cx.add_window_view(fixture_window);
    view.update_in(cx, |view, window, cx| {
        let tab = install(view, window, cx);
        let first = view.core_terminals.tabs[&tab].active.clone();
        view.core_terminals
            .tabs
            .get_mut(&tab)
            .unwrap()
            .set_zoom(Some(first));
        assert!(view.core_pane_command(Command::FocusRight, window, cx));
        let shown = &view.core_terminals.tabs[&tab];
        assert_eq!(shown.active.as_str(), "pager-second");
        assert!(!shown.zoom.is_open());
    });
}

#[gpui::test]
fn desktop_next_pane_keeps_the_existing_command_route(cx: &mut TestAppContext) {
    let _desktop = crate::ipad_ui::desktop();
    let (view, cx) = cx.add_window_view(fixture_window);
    view.update_in(cx, |view, window, cx| {
        let tab = install(view, window, cx);
        let active = view.core_terminals.tabs[&tab].active.clone();
        assert!(!view.core_pane_command(Command::NextPane, window, cx));
        assert!(!view.core_pane_command(Command::PreviousPane, window, cx));
        assert_eq!(view.core_terminals.tabs[&tab].active, active);
    });
}

fn touch(cx: &mut VisualTestContext, id: u64, phase: TouchPhase, position: Point<Pixels>) {
    cx.simulate_event(TouchEvent {
        id: TouchId(id),
        phase,
        position,
        predicted_position: None,
        force: None,
    });
}

#[gpui::test]
fn pager_strip_claims_one_finger_swipes_and_raw_dot_taps(cx: &mut TestAppContext) {
    let _ipad = crate::ipad_ui::ipad_layout();
    let (view, cx) = cx.add_window_view(fixture_window);
    let tab = view.update_in(cx, install);
    cx.simulate_resize(size(px(1194.), px(834.)));
    cx.simulate_window_visual_viewport_change(
        cx.window_handle(),
        Bounds::new(point(px(0.), px(0.)), size(px(1194.), px(534.))),
    );
    cx.update(|window, cx| window.draw(cx).clear(cx));
    let start = cx.debug_bounds("ipad-pane-pager").unwrap().center();
    let end = start - point(px(60.), px(0.));
    touch(cx, 1, TouchPhase::Started, start);
    touch(cx, 1, TouchPhase::Moved, end);
    touch(cx, 1, TouchPhase::Ended, end);
    view.read_with(cx, |view, _| {
        let shown = &view.core_terminals.tabs[&tab];
        assert_eq!(shown.active.as_str(), "pager-second");
        assert!(shown.zoom.is_open());
        assert_eq!(shown.zoomed.as_ref(), Some(&shown.active));
    });
    cx.update(|window, cx| window.draw(cx).clear(cx));
    let dot = cx.debug_bounds("ipad-pane-dot-0").unwrap().center();
    touch(cx, 2, TouchPhase::Started, dot);
    touch(cx, 2, TouchPhase::Ended, dot);
    assert_eq!(
        view.read_with(cx, |view, _| view.core_terminals.tabs[&tab]
            .active
            .as_str()
            .to_owned()),
        "pager-first"
    );
    cx.update(|window, cx| window.draw(cx).clear(cx));
    let start = cx.debug_bounds("ipad-pane-pager").unwrap().center();
    touch(cx, 3, TouchPhase::Started, start);
    touch(cx, 3, TouchPhase::Moved, start - point(px(60.), px(0.)));
    touch(cx, 3, TouchPhase::Cancelled, start - point(px(60.), px(0.)));
    assert_eq!(
        view.read_with(cx, |view, _| view.core_terminals.tabs[&tab]
            .active
            .as_str()
            .to_owned()),
        "pager-first"
    );
}
