#![allow(clippy::unwrap_used)]

use super::render;
use gpui::{Context, IntoElement, Render, TestAppContext, Window, div, prelude::*, px, size};

struct Fixture {
    debug: bool,
    worktree: bool,
    pr: &'static str,
}

impl Render for Fixture {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .flex()
            .flex_col()
            .children(render(
                self.debug,
                self.worktree,
                "worktree/a-very-long-branch-name-that-must-not-push-the-pr-out-of-the-window",
                self.pr,
            ))
            .child(div().debug_selector(|| "body".into()).flex_1().min_h_0())
    }
}

#[gpui::test]
fn banner_reserves_space_only_for_debug_worktrees(cx: &mut TestAppContext) {
    let (view, cx) = cx.add_window_view(|_, _| Fixture {
        debug: false,
        worktree: false,
        pr: "",
    });
    for (debug, worktree) in [(false, false), (false, true), (true, false), (true, true)] {
        let visible = debug && worktree;
        for pr in ["", "12345"] {
            for width in [360., 640., 1200.] {
                view.update(cx, |view, cx| {
                    view.debug = debug;
                    view.worktree = worktree;
                    view.pr = pr;
                    cx.notify();
                });
                cx.simulate_resize(size(px(width), px(400.)));
                cx.run_until_parked();
                cx.update(|window, cx| {
                    window.refresh();
                    let _ = window.draw(cx);
                });
                let body = cx.debug_bounds("body").unwrap();
                assert_eq!(body.top(), px(if visible { 22. } else { 0. }));
                assert_eq!(body.bottom(), px(400.));
                if visible {
                    let banner = cx.debug_bounds("worktree-banner").unwrap();
                    let branch = cx.debug_bounds("worktree-branch").unwrap();
                    assert_eq!(banner.size, size(px(width), px(22.)));
                    assert_eq!(branch.left(), banner.left() + px(12.));
                    assert!(branch.right() <= banner.right());
                    if !pr.is_empty() {
                        let pr = cx.debug_bounds("worktree-pr").unwrap();
                        assert!(pr.left() >= branch.right());
                        assert!(pr.right() <= banner.right());
                        cx.simulate_click(pr.center(), Default::default());
                        assert_eq!(
                            cx.opened_url().as_deref(),
                            Some("https://github.com/sheprd-labs/sheprd/pull/12345")
                        );
                    } else {
                        assert!(cx.debug_bounds("worktree-pr").is_none());
                    }
                } else {
                    assert!(cx.debug_bounds("worktree-banner").is_none());
                }
            }
        }
    }
}
