//! Shared headings for the sidebar's workspace and agent sections.

use crate::{
    HerdrWindow,
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
    /// Touch entries use compact icons; desktop headings keep their field.
    pub(super) fn spaces_header(
        &self,
        (devices, searchable, tree_search): (bool, bool, bool),
        look: SidebarLook,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> Div {
        let font = &self.config.sidebar;
        let theme = &self.theme;
        if crate::ipad_ui::layout() {
            return div()
                .debug_selector(|| "header-spaces".into())
                .h(px(crate::ipad_ui::metrics::TOUCH))
                .flex_none()
                .row()
                .items_center()
                .ps(px(look.content_x()))
                .child(
                    div()
                        .debug_selector(|| "header-label-spaces".into())
                        .flex_1()
                        .min_w_0()
                        .truncate()
                        .child(label_text(
                            &look.header_label(&fl!("sidebar-header-spaces")),
                        )),
                )
                .when(searchable || tree_search, |row| {
                    row.child(self.sidebar_search_trigger("sidebar-search-open", cx))
                })
                .child(self.sidebar_new_space_trigger("sidebar-new-space", cx));
        }
        searchable
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
            })
    }
}
