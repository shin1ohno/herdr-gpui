use crate::config::{FontConfig, Theme, corners};
use gpui::{Div, SharedString, div, prelude::*, px, rgb};

/// Shortcut glyphs are data, not translatable prose. Chips never take focus.
pub(crate) fn kbd(text: impl Into<SharedString>, theme: &Theme, font: &FontConfig) -> Div {
    // i18n: data
    div()
        .debug_selector(|| "ipad-kbd".into())
        .flex_none()
        .px(px(5.))
        .py(px(2.))
        .border_1()
        .border_color(rgb(theme.active))
        .rounded(px(corners::SMALL))
        .font(font.font())
        .text_size(px(font.size))
        .text_color(rgb(theme.subtext()))
        .child(text.into())
}
