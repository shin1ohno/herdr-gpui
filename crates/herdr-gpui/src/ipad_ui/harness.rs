use crate::direction::Logical;
use crate::{config::Theme, icons::AgentIcon};
use gpui::{prelude::*, *};

/// A harness mark and its bounded label, sized by the surface using it.
pub(crate) fn row(icon: AgentIcon, size: f32, text: SharedString, theme: &Theme) -> Div {
    // i18n: data
    div()
        .row()
        .items_center()
        .min_w_0()
        .gap(px(5.))
        .child(
            div()
                .size(px(size))
                .flex_none()
                .child(icon.element(size, theme.subtext())),
        )
        .child(
            div()
                .min_w_0()
                .truncate()
                .child(crate::sidebar::label_text(&text)),
        )
}
