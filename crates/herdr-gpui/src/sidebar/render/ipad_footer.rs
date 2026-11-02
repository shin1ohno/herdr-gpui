use crate::direction::Logical;
use crate::{
    Command, HerdrWindow,
    i18n::fl,
    ipad_ui::{kbd, metrics::TOUCH},
};
use gpui::{prelude::*, *};

impl HerdrWindow {
    pub(super) fn ipad_sidebar_footer(&self, window: &Window, cx: &mut Context<Self>) -> Div {
        let font = &self.config.sidebar;
        let theme = &self.theme;
        let hint = self
            .ipad
            .keyboard
            .hardware(window)
            .then(|| self.shortcut_hint(Command::Workspace))
            .flatten();
        div()
            .debug_selector(|| "ipad-sidebar-footer".into())
            .flex_none()
            .m(px(12.))
            .flex()
            .flex_col()
            .child(
                div()
                    .id("new-workspace")
                    .debug_selector(|| "new-workspace".into())
                    .map(|button| self.sidebar_button(button, fl!("ipad-sidebar-new-space")))
                    .h(px(TOUCH))
                    .flex_none()
                    .row()
                    .items_center()
                    .justify_between()
                    .cursor_pointer()
                    .child(fl!("ipad-sidebar-new-space"))
                    .children(hint.map(|hint| {
                        kbd(hint, theme, font).debug_selector(|| "ipad-new-space-shortcut".into())
                    }))
                    .on_click(cx.listener(|this, _, window, cx| {
                        this.command(Command::Workspace, window, cx)
                    })),
            )
            .child(
                div()
                    .id("ipad-sidebar-settings")
                    .debug_selector(|| "ipad-sidebar-settings".into())
                    .map(|button| self.sidebar_button(button, fl!("ipad-sidebar-settings")))
                    .h(px(TOUCH))
                    .flex_none()
                    .row()
                    .items_center()
                    .cursor_pointer()
                    .text_color(rgb(theme.subtext()))
                    .child(fl!("ipad-sidebar-settings"))
                    .on_click(cx.listener(|this, _, window, cx| {
                        this.command(Command::Settings, window, cx)
                    })),
            )
    }
}
