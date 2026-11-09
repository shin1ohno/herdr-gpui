use gpui::{prelude::*, *};

/// Debug worktree builds hang this strip at the window's foot.
const HEIGHT: f32 = 22.;

pub(super) fn pull_request_url(pr: &str) -> String {
    format!("{}/pull/{pr}", crate::about::REPOSITORY)
}

pub(super) fn render(debug: bool, worktree: bool, branch: &str, pr: &str) -> Option<Div> {
    (debug && worktree).then(|| {
        div()
            .debug_selector(|| "worktree-banner".into())
            .flex()
            .flex_none()
            .items_center()
            .gap(px(10.))
            .px(px(12.))
            .w_full()
            .h(px(HEIGHT))
            .overflow_hidden()
            .bg(rgb(0xf6c453))
            .text_color(rgb(0x402b08))
            .text_size(px(12.))
            .child(
                div()
                    .debug_selector(|| "worktree-branch".into())
                    .flex_1()
                    .min_w_0()
                    .truncate()
                    .child(branch.to_owned()),
            )
            .when(!pr.is_empty(), |row| {
                let url = pull_request_url(pr);
                row.child(
                    div()
                        .id("worktree-pr")
                        .debug_selector(|| "worktree-pr".into())
                        .flex_none()
                        .cursor_pointer()
                        .hover(|style| style.underline())
                        .child(crate::i18n::fl!("titlebar-pull-request", number = pr))
                        .on_click(move |_, _, cx| cx.open_url(&url)),
                )
            })
    })
}

#[cfg(test)]
mod tests;
