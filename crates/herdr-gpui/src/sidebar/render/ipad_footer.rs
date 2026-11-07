use crate::direction::Logical;
use crate::{Command, HerdrWindow, i18n::fl, ipad_ui::metrics::TOUCH};
use gpui::{prelude::*, *};

impl HerdrWindow {
    pub(super) fn ipad_sidebar_footer(&self, cx: &mut Context<Self>) -> Div {
        let theme = &self.theme;
        div()
            .debug_selector(|| "ipad-sidebar-footer".into())
            .flex_none()
            .m(px(12.))
            .flex()
            .flex_col()
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
