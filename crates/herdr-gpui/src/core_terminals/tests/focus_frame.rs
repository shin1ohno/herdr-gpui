use crate::{
    HerdrWindow,
    browser::{Location, Slot, Store, Tab},
    core_terminals::{
        TabPanes,
        panes::{Axis, Panes},
        view::CoreTerminalView,
        worker::Link,
    },
    sidebar::layout_tests::fixture_window,
};
use core::prelude::v1::test;
use gpui::{prelude::*, *};
use herdr_backend::TerminalId;

struct Surface {
    view: Entity<HerdrWindow>,
    tab: Tab,
}

impl Render for Surface {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.view.update(cx, |view, cx| {
            let slot: Slot = view.group_slots()[0];
            div()
                .size_full()
                .flex()
                .child(view.render_core_terminal_tab(slot, &self.tab, cx))
        })
    }
}

fn fixture(cx: &mut TestAppContext) -> (Entity<Surface>, &mut VisualTestContext) {
    cx.add_window_view(|window, cx| {
        let view = cx.new(|cx| {
            let mut view = fixture_window(window, cx);
            // Select an actually supported target without using a personal core.
            view.endpoints[0].connection.target = [
                herdr_client::ConnectTarget::Local,
                herdr_client::ConnectTarget::Ssh {
                    target: "me@core.test".into(),
                    session: "test".into(),
                },
            ]
            .into_iter()
            .find(crate::core_terminals::supports_core_target)
            .unwrap();
            view.theme = crate::config::Theme::builtin("Nord").unwrap();
            view
        });
        let tab = view.update(cx, |view, cx| {
            let first = TerminalId::new("t1").unwrap();
            let second = TerminalId::new("t2").unwrap();
            let mut panes = Panes::single(first.clone());
            panes.split(&first, Axis::Row, second.clone()).unwrap();
            let scope = crate::browser::scope(&view.endpoints[0]);
            let id = Store::update(cx, |store| {
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
                .insert(id, TabPanes::new(panes, false));
            for terminal in [first, second] {
                let (link, _commands) = Link::recording();
                let look = view.core_look();
                let painter = view.painter.clone();
                let core = cx.new(|cx| {
                    CoreTerminalView::new(terminal.clone(), link, look, painter, false, cx)
                });
                view.core_terminals.views.insert((id, terminal), core);
            }
            cx.global::<Store>().get(id).unwrap().clone()
        });
        cx.observe(&view, |_, _, cx| cx.notify()).detach();
        Surface { view, tab }
    })
}

fn draw(cx: &mut VisualTestContext) {
    cx.update(|window, cx| window.draw(cx).clear(cx));
}

#[gpui::test]
fn ipad_core_focus_frame_moves_without_consuming_cell_space(cx: &mut TestAppContext) {
    let _ipad = crate::ipad_ui::ipad_layout();
    let (surface, cx) = fixture(cx);
    cx.simulate_resize(size(px(800.), px(500.)));
    draw(cx);
    let id = surface.read_with(cx, |surface, _| u64::from(surface.tab.id));
    let selector: &'static str = Box::leak(format!("core-focus-frame-{id}-t1").into_boxed_str());
    let pane: &'static str = Box::leak(format!("core-pane-{id}-t1").into_boxed_str());
    let first_bounds = cx.debug_bounds(pane).unwrap();
    assert_eq!(cx.debug_bounds(selector).unwrap(), first_bounds);
    cx.update(|window, _| {
        let theme = crate::config::Theme::builtin("Nord").unwrap();
        let accent: Hsla = rgb(crate::ipad_ui::accent(&theme)).into();
        assert!(
            window
                .painted_quads()
                .iter()
                .any(|quad| quad.border_color == accent
                    && quad.border_widths.top == px(2.).scale(window.scale_factor())
                    && quad.border_widths.left == px(2.).scale(window.scale_factor()))
        );
    });
    let view = surface.read_with(cx, |surface, _| surface.view.clone());
    view.update(cx, |view, cx| {
        view.core_terminals.tabs.values_mut().next().unwrap().active =
            TerminalId::new("t2").unwrap();
        cx.notify();
    });
    draw(cx);
    assert!(cx.debug_bounds(selector).is_none());
    assert_eq!(cx.debug_bounds(pane).unwrap(), first_bounds);
    assert!(
        cx.debug_bounds(Box::leak(
            format!("core-focus-frame-{id}-t2").into_boxed_str()
        ))
        .is_some()
    );
    view.update(cx, |view, cx| {
        let shown = view.core_terminals.tabs.values_mut().next().unwrap();
        shown.panes.remove(&TerminalId::new("t1").unwrap());
        cx.notify();
    });
    draw(cx);
    assert!(
        cx.debug_bounds(Box::leak(
            format!("core-focus-frame-{id}-t2").into_boxed_str()
        ))
        .is_none()
    );
}

#[gpui::test]
fn desktop_core_panes_gain_no_ipad_frame(cx: &mut TestAppContext) {
    let _desktop = crate::ipad_ui::desktop();
    let (surface, cx) = fixture(cx);
    draw(cx);
    let id = surface.read_with(cx, |surface, _| u64::from(surface.tab.id));
    assert!(
        cx.debug_bounds(Box::leak(
            format!("core-focus-frame-{id}-t1").into_boxed_str()
        ))
        .is_none()
    );
}

#[gpui::test]
fn keyboard_action_focuses_the_shown_core_pane_without_focusing_herdr_1(cx: &mut TestAppContext) {
    let _ipad = crate::ipad_ui::ipad_layout();
    let (surface, cx) = fixture(cx);
    let (view, tab) = surface.read_with(cx, |surface, _| (surface.view.clone(), surface.tab.id));
    view.update_in(cx, |view, window, cx| {
        let snapshot = serde_json::from_str(include_str!(
            "../../../../herdr-protocol/tests/fixtures/endpoint-snapshot-v1.json"
        ))
        .unwrap();
        view.live.snapshot = Some(std::sync::Arc::new(snapshot));
        let layout = view.ensure_layout().unwrap();
        let group = layout.active();
        layout.show(group, crate::browser::Pick::Page(tab));
        window.focus(&view.focus, cx);
        assert!(view.request_core_terminal_keyboard(window, cx));
        let terminal = view.core_terminals.tabs.get(&tab).unwrap().active.clone();
        let core = view.core_terminals.views.get(&(tab, terminal)).unwrap();
        assert!(core.read(cx).focus_handle().is_focused(window));
        assert!(!view.focus.is_focused(window));
        // A Core page waiting for its view is still Core's input destination.
        view.core_terminals.views.clear();
        window.focus(&view.focus, cx);
        assert!(view.request_core_terminal_keyboard(window, cx));
        view.ensure_layout()
            .unwrap()
            .show(group, crate::browser::Pick::Herdr("w1:t1".into()));
        assert!(!view.request_core_terminal_keyboard(window, cx));
    });
}
