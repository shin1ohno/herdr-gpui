use super::*;

#[derive(Default)]
struct NestedFixture {
    opened: Vec<Point<Pixels>>,
    tooltip_builds: Rc<Cell<usize>>,
}

impl Render for NestedFixture {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let builds = self.tooltip_builds.clone();
        let row = div()
            .id("nested-context-row")
            .ml(px(40.))
            .mt(px(40.))
            .w(px(100.))
            .h(px(100.))
            .child(
                div()
                    .id("nested-context-tooltip")
                    .size_full()
                    .tooltip(move |_, cx| {
                        builds.set(builds.get() + 1);
                        cx.new(|_| Empty).into()
                    }),
            );
        div().relative().size(px(200.)).child(context_menu(
            row,
            cx.listener(|this, event: &ContextMenuEvent, _, _| {
                this.opened.push(event.position);
            }),
        ))
    }
}

#[gpui::test]
fn context_menu_claims_a_descendants_tooltip_press_at_the_exact_row_bounds(
    cx: &mut TestAppContext,
) {
    let (view, cx) = cx.add_window_view(|_, _| NestedFixture::default());
    cx.update(|window, cx| window.draw(cx).clear(cx));
    let outside = point(px(20.), px(20.));
    assert!(!long_press(cx, TouchPhase::Started, outside));
    view.read_with(cx, |view, _| assert!(view.opened.is_empty()));

    let inside = point(px(60.), px(60.));
    assert!(long_press(cx, TouchPhase::Started, inside));
    view.read_with(cx, |view, _| {
        assert_eq!(view.opened, [inside]);
        assert_eq!(view.tooltip_builds.get(), 0);
    });
}
