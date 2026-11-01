#![allow(clippy::unwrap_used)]

use super::*;
use core::prelude::v1::test;
use std::cell::Cell;

mod long_press;
mod review_gestures;

#[derive(Default)]
struct Fixture {
    opened: Vec<(Point<Pixels>, bool)>,
    clicks: usize,
    tooltip_builds: Rc<Cell<usize>>,
    covered: bool,
}

impl Render for Fixture {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let builds = self.tooltip_builds.clone();
        let row = div()
            .id("context-row")
            .relative()
            .w(px(200.))
            .h(px(200.))
            .tooltip(move |_, cx| {
                builds.set(builds.get() + 1);
                cx.new(|_| Empty).into()
            })
            .on_click(cx.listener(|this, _, _, _| this.clicks += 1));
        div()
            .relative()
            .w(px(200.))
            .h(px(100.))
            .overflow_hidden()
            .child(context_menu(
                row,
                cx.listener(|this, event: &ContextMenuEvent, _, _| {
                    this.opened.push((event.position, event.mouse.is_some()));
                }),
            ))
            .when(self.covered, |root| {
                root.child(div().absolute().inset_0().occlude())
            })
    }
}

fn touch(cx: &mut VisualTestContext, phase: TouchPhase, position: Point<Pixels>) {
    cx.update(|window, cx| {
        window.dispatch_event(
            TouchEvent {
                id: TouchId(1),
                phase,
                position,
                predicted_position: None,
                force: None,
            }
            .to_platform_input(),
            cx,
        );
    });
}

fn long_press(cx: &mut VisualTestContext, phase: TouchPhase, position: Point<Pixels>) -> bool {
    cx.update(|window, cx| {
        window
            .dispatch_event(
                LongPressEvent {
                    phase,
                    start_position: position,
                    position,
                }
                .to_platform_input(),
                cx,
            )
            .default_prevented
    })
}
