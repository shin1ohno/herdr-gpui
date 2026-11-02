//! Shared headings for the sidebar's workspace and agent sections.

use crate::{
    Command, HerdrWindow,
    config::{FontConfig, Theme},
    direction::Logical,
    i18n::fl,
    sidebar::{label_text, layout::SidebarLook, line_height},
};
use gpui::{prelude::*, *};

/// A section's heading: `key` names its debug selectors, `label` is its
/// text, cased as the look's headers are.
pub(in crate::sidebar) fn header(
    key: &'static str,
    label: &str,
    font: &FontConfig,
    theme: &Theme,
    look: SidebarLook,
) -> Div {
    div()
        .debug_selector(|| format!("header-{key}"))
        .flex_none()
        .h(px(line_height(font) + 2. * look.density.header_padding()))
        .px(px(look.content_x()))
        .row()
        .items_center()
        .text_size(px(font.size))
        .text_color(rgb(theme.muted))
        .when(crate::ipad_ui::layout(), |header| {
            header
                .h(px(crate::ipad_ui::metrics::TOUCH))
                .text_color(rgb(theme.subtext()))
        })
        .child(
            div()
                .debug_selector(|| format!("header-label-{key}"))
                .child(label_text(&look.header_label(label))),
        )
}

impl HerdrWindow {
    /// Preserve the searchable heading; append the resolved hardware hint to
    /// the same row so neither searching nor its accessibility is replaced.
    pub(super) fn spaces_header(
        &self,
        (devices, searchable, tree_search): (bool, bool, bool),
        look: SidebarLook,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> Div {
        let font = &self.config.sidebar;
        let theme = &self.theme;
        let heading = searchable
            .then_some(crate::sidebar::search::Heading::Spaces)
            .or(tree_search.then_some(crate::sidebar::search::Heading::DeviceTree))
            .and_then(|heading| self.search_heading(heading, look, window, cx))
            .unwrap_or_else(|| {
                let (key, label) = if devices {
                    ("devices", fl!("sidebar-header-devices"))
                } else {
                    ("spaces", fl!("sidebar-header-spaces"))
                };
                header(key, &label, font, theme, look)
            });
        heading.when(crate::ipad_ui::layout(), |heading| {
            heading
                .h(px(crate::ipad_ui::metrics::TOUCH))
                .justify_between()
                .gap(px(6.))
                .children(
                    self.ipad
                        .keyboard
                        .hardware(window)
                        .then(|| self.shortcut_hint(Command::WorkspacePicker))
                        .flatten()
                        .map(|hint| {
                            crate::ipad_ui::kbd(hint, theme, font)
                                .debug_selector(|| "ipad-spaces-shortcut".into())
                        }),
                )
        })
    }
}
