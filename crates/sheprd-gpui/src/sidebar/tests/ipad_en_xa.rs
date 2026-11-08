use super::*;
use crate::{
    SheprdWindow,
    i18n::leak_tests::{assert_translated, draw, uncut, under_en_xa},
};
use gpui::{AppContext, Context, Entity, IntoElement, Render, Window, px, size};
use std::sync::Arc;

struct Sidebar(Entity<SheprdWindow>);

fn measured_without_cuts(shown: Vec<String>) -> Vec<String> {
    // The shared helper compares byte lengths. Replacing only a closing ']'
    // with the three-byte ellipsis makes a valid cut longer than its source.
    // Accept it only when this test actually measured the complete message.
    uncut(shown.clone())
        .into_iter()
        .filter(|line| {
            line.strip_suffix('…').is_none_or(|stem| {
                !shown
                    .iter()
                    .any(|whole| sheprd_i18n::pseudo::is_marked(whole) && whole.starts_with(stem))
            })
        })
        .collect()
}

impl Render for Sidebar {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.0.update(cx, |view, cx| {
            let indicators = Indicators::new(None, StatusIcons::Dots, false, false, &view.theme);
            view.render_sidebar(indicators, window, cx)
        })
    }
}

#[test]
fn ipad_sidebar_prose_is_translated_and_names_are_data() {
    under_en_xa("iPad sidebar", |cx, recorder| {
        let _ipad = crate::ipad_ui::ipad_layout();
        let (surface, cx) = cx.add_window_view(|window, cx| {
            let view = cx.new(|cx| {
                let mut view = layout_tests::fixture_window(window, cx);
                let mut snapshot = layout_tests::snapshot(1);
                snapshot.agents.truncate(1);
                snapshot.agents[0].agent_status = AgentStatus::Blocked;
                view.live.snapshot = Some(Arc::new(snapshot));
                view.config.show_sidebar_search = false;
                view.sidebar_width = Some(480.);
                view
            });
            cx.observe(&view, |_, _, cx| cx.notify()).detach();
            Sidebar(view)
        });
        cx.simulate_resize(size(px(1194.), px(834.)));
        draw(cx);
        let mut seen = recorder.take();
        let data = [
            "Local".into(),
            "local".into(),
            "herdr".into(),
            "main".into(),
            "review".into(),
        ];
        assert_translated("iPad sidebar", &uncut(seen.clone()), &data);
        let view = surface.read_with(cx, |surface, _| surface.0.clone());
        view.update(cx, |view, cx| {
            view.sidebar_width = Some(300.);
            cx.notify();
        });
        draw(cx);
        // The narrow frame reuses shaped text. Keep the real wide frame's
        // full strings so uncut can recognize its ellipsis measurements.
        seen.extend(recorder.take());
        assert_translated(
            "iPad sidebar wide and narrow",
            &measured_without_cuts(seen),
            &data,
        );
    });
}
