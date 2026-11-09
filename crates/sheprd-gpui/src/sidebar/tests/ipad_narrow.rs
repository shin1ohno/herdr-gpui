use super::*;
use crate::{SheprdWindow, config::AgentsPlacement, ipad_ui::metrics::DOT};
use gpui::{
    AppContext, Context, Entity, IntoElement, Render, TestAppContext, VisualTestContext, Window,
    px, size,
};
use std::sync::Arc;

#[test]
fn ipad_space_widths_keep_natural_extras_on_a_wide_line() {
    let widths = super::super::layouts::Ipad::space_widths(240., 80., Some(36.), 48.);
    assert_eq!(
        (widths.name, widths.badge, widths.count),
        (128., Some(36.), Some(48.))
    );
}

#[test]
fn ipad_space_widths_drop_count_before_badge() {
    let split = |width, name, badge, count| {
        let widths = super::super::layouts::Ipad::space_widths(width, name, badge, count);
        (widths.name, widths.badge, widths.count)
    };
    assert_eq!(split(160., 80., Some(36.), 48.), (102., Some(36.), None));
    assert_eq!(split(100., 80., Some(36.), 48.), (42., Some(36.), None));
    assert_eq!(split(99., 80., Some(36.), 48.), (83., None, None));
    assert_eq!(
        split(192., 80., Some(36.), 48.),
        (80., Some(36.), Some(48.))
    );
}

#[test]
fn ipad_space_widths_floor_uses_room_after_the_dot_and_gap() {
    let widths = super::super::layouts::Ipad::space_widths(270., 200., Some(72.), 42.);
    assert_eq!(widths.badge, Some(72.));
    assert_eq!(widths.count, Some(42.));
    assert_eq!(widths.name, 128.);
    assert!(widths.name >= (270. - DOT - 6.) / 2.);
}

#[test]
fn ipad_space_widths_preserve_name_floor_and_account_for_only_shown_gaps() {
    let split = |width, name, badge, count| {
        let widths = super::super::layouts::Ipad::space_widths(width, name, badge, count);
        (widths.name, widths.badge, widths.count)
    };
    for width in [40., 72., 100., 160., 192., 240., 300.] {
        for natural in [0., 8., 80., 600.] {
            for badge in [None, Some(36.), Some(200.)] {
                let (name, shown_badge, shown_count) = split(width, natural, badge, 48.);
                let floor = natural.min((width - DOT - 6.).max(0.) / 2.);
                assert!(name >= floor, "{width}: {name}");
                assert!(shown_badge.is_none() || shown_badge == badge);
                assert!(shown_count.is_none() || shown_count == Some(48.));
                let used = DOT
                    + 6.
                    + name
                    + shown_badge.map_or(0., |width| width + 6.)
                    + shown_count.map_or(0., |width| width + 6.);
                assert_eq!(used, width);
            }
        }
    }
    assert_eq!(split(100., 8., None, 48.), (30., None, Some(48.)));
}

fn fixture(window: &mut Window, cx: &mut Context<SheprdWindow>) -> SheprdWindow {
    let mut view = layout_tests::fixture_window(window, cx);
    let mut snapshot = layout_tests::snapshot(6);
    snapshot.agents[0].agent_status = AgentStatus::Blocked;
    snapshot.workspaces[0].agent_status = AgentStatus::Blocked;
    view.live.snapshot = Some(Arc::new(snapshot));
    view.live.status = crate::state::ConnectionStatus::Connected;
    view.config.agents_placement = AgentsPlacement::Panel;
    view.config.show_agents = true;
    view.config.show_sidebar_search = false;
    view.config.sidebar_style.activity_bar = false;
    view.sidebar_split = Some(0.72);
    view
}

struct NarrowSidebar(Entity<SheprdWindow>);

impl Render for NarrowSidebar {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.0.update(cx, |view, cx| {
            let indicators = Indicators::new(None, StatusIcons::Dots, false, false, &view.theme);
            view.render_sidebar(indicators, window, cx)
        })
    }
}

// These tests measure narrow sidebar rows independently of the compact shell,
// which replaces the inline column below 560pt.
fn narrow_sidebar(cx: &mut TestAppContext) -> (Entity<SheprdWindow>, &mut VisualTestContext) {
    let (surface, cx) =
        cx.add_window_view(|window, cx| NarrowSidebar(cx.new(|cx| fixture(window, cx))));
    let view = surface.read_with(cx, |surface, _| surface.0.clone());
    (view, cx)
}

#[gpui::test]
fn ipad_narrow_long_blocked_space_keeps_both_extras_at_default_width(cx: &mut TestAppContext) {
    let _ipad = crate::ipad_ui::ipad_layout();
    let (_view, cx) = cx.add_window_view(|window, cx| {
        let mut view = fixture(window, cx);
        let snapshot = Arc::make_mut(view.live.snapshot.as_mut().unwrap());
        let mut space = snapshot.workspaces[0].clone();
        space.workspace_id = "w-long".into();
        space.active_tab_id = "long-t0".into();
        space.label = "a-long-blocked-space-name".into();
        space.focused = false;
        snapshot.workspaces.push(space);
        let tabs = snapshot.tabs.clone();
        snapshot
            .tabs
            .extend(tabs.into_iter().take(1).map(|mut tab| {
                tab.tab_id = format!("long-{}", tab.tab_id);
                tab.workspace_id = "w-long".into();
                tab
            }));
        view
    });
    cx.simulate_resize(size(px(1194.), px(834.)));
    cx.update(|window, cx| layout_tests::full_draw(window, cx).clear(cx));
    assert_eq!(cx.debug_bounds("sidebar").unwrap().size.width, px(300.));
    let count = cx.debug_bounds("ipad-tab-count-w-long");
    let badge = cx.debug_bounds("ipad-needs-you-w-long");
    assert!(count.is_some(), "long blocked space lost its tab count");
    assert!(badge.is_some(), "long blocked space lost its badge");
    let row = cx.debug_bounds("row-a-long-blocked-space-name").unwrap();
    assert!(count.unwrap().right() <= badge.unwrap().left());
    assert!(badge.unwrap().right() <= row.right());
}

#[gpui::test]
fn ipad_narrow_one_character_agent_meta_is_rendered_when_whole(cx: &mut TestAppContext) {
    let _ipad = crate::ipad_ui::ipad_layout();
    let (_view, cx) = cx.add_window_view(|window, cx| {
        let mut view = fixture(window, cx);
        Arc::make_mut(view.live.snapshot.as_mut().unwrap()).workspaces[1].label = "w".into();
        view
    });
    cx.simulate_resize(size(px(1194.), px(834.)));
    cx.update(|window, cx| layout_tests::full_draw(window, cx).clear(cx));
    let meta = cx.debug_bounds("ipad-agent-meta-agent-p1");
    assert!(meta.is_some(), "a whole one-character meta should be shown");
    let meta = meta.unwrap();
    assert!(meta.size.width > px(0.));
    assert!(meta.right() <= cx.debug_bounds("row-agent-p1").unwrap().right());
}

#[gpui::test]
fn ipad_narrow_space_name_keeps_its_floor_and_extras_stay_inside(cx: &mut TestAppContext) {
    let _ipad = crate::ipad_ui::ipad_layout();
    let (view, cx) = narrow_sidebar(cx);
    cx.simulate_resize(size(px(375.), px(834.)));
    cx.update(|window, cx| layout_tests::full_draw(window, cx).clear(cx));
    assert_eq!(cx.debug_bounds("sidebar").unwrap().size.width, px(135.));
    let name = cx.debug_bounds("ipad-space-name-w0").unwrap();
    let row = cx.debug_bounds("row-herdr").unwrap();
    let floor = view.read_with(cx, |view, _| {
        let look = super::super::layout::for_config(&view.config, view.fill());
        let line = look.content_width(135.);
        let natural = "herdr".chars().count() as f32 * 14. * view.config.sidebar.size / 12. * 0.62;
        natural.min(line / 2.)
    });
    assert!(name.size.width > px(0.));
    assert!(
        name.size.width >= px(floor),
        "space name {name:?} is narrower than its {floor}pt floor"
    );
    for selector in ["ipad-tab-count-w0", "ipad-needs-you-w0"] {
        if let Some(extra) = cx.debug_bounds(selector) {
            assert!(extra.left() >= row.left(), "{selector}: {extra:?}");
            assert!(extra.right() <= row.right(), "{selector}: {extra:?}");
        }
    }
}

#[gpui::test]
fn ipad_narrow_agent_names_keep_their_floor(cx: &mut TestAppContext) {
    let _ipad = crate::ipad_ui::ipad_layout();
    let (view, cx) = narrow_sidebar(cx);
    cx.simulate_resize(size(px(375.), px(834.)));
    cx.update(|window, cx| layout_tests::full_draw(window, cx).clear(cx));
    let (glyph, floors) = view.read_with(cx, |view, _| {
        let look = super::super::layout::for_config(&view.config, view.fill());
        let room = (look.content_width(135.) - DOT - 18. - 5. - 2. * 6.).max(0.);
        let glyph = view.config.sidebar.size * 0.62;
        let snapshot = view.live.snapshot.as_deref().unwrap();
        let floors: Vec<_> = snapshot
            .agents
            .iter()
            .map(|agent| {
                let natural =
                    crate::sidebar::agent_label(agent, snapshot).chars().count() as f32 * glyph;
                (agent.pane_id.clone(), natural.min(room / 2.))
            })
            .collect();
        (glyph, floors)
    });
    for (pane, floor) in floors {
        let selector = format!("ipad-agent-name-agent-{pane}").leak();
        let name = cx.debug_bounds(selector).unwrap();
        assert!(
            name.size.width - px(18. + 5.) >= px(floor),
            "{selector}: {name:?} leaves less than its {floor}pt text floor"
        );
        if let Some(meta) = cx.debug_bounds(format!("ipad-agent-meta-agent-{pane}").leak()) {
            assert!(meta.size.width >= px(2. * glyph));
            assert!(
                meta.right()
                    <= cx
                        .debug_bounds(format!("row-agent-{pane}").leak())
                        .unwrap()
                        .right()
            );
        }
    }
}

#[gpui::test]
fn ipad_narrow_agent_meta_under_two_glyphs_is_not_rendered(cx: &mut TestAppContext) {
    let _ipad = crate::ipad_ui::ipad_layout();
    let (_view, cx) = narrow_sidebar(cx);
    cx.simulate_resize(size(px(320.), px(834.)));
    cx.update(|window, cx| layout_tests::full_draw(window, cx).clear(cx));
    assert!(cx.debug_bounds("ipad-agent-meta-agent-p0").is_none());
    assert!(cx.debug_bounds("ipad-agent-meta-agent-p1").is_none());
    let name = cx.debug_bounds("ipad-agent-name-agent-p0").unwrap();
    let row = cx.debug_bounds("row-agent-p0").unwrap();
    assert!(name.size.width > px(18. + 5.));
    assert!(name.right() <= row.right());
}
