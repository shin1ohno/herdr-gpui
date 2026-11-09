//! The iPad SSH sheet: a fixed header above the scrolling form.

use super::actions::step_actions;
use crate::{SheprdWindow, i18n::fl, search_input::SearchInput};
use gpui::{prelude::*, *};

const SHEET_HEADER: f32 = 60.;
const FIELD: f32 = 48.;

impl SheprdWindow {
    pub(super) fn add_device_sheet(&self, pane: Div, cx: &mut Context<Self>) -> Div {
        let setup = self.menu.device_setup.as_ref();
        let actions = setup.map(|setup| step_actions(&setup.step));
        let cancel_step = actions.as_ref().is_some_and(|actions| actions.cancel);
        let mut header = div()
            .debug_selector(|| "device-setup-header".into())
            .flex_none()
            .h(px(SHEET_HEADER))
            .px(px(12.))
            .border_b_1()
            .border_color(rgb(self.theme.active))
            .flex()
            .items_center()
            .gap(px(8.))
            .child(
                self.device_action_button(
                    "device-setup-sheet-cancel",
                    fl!("add-device-cancel"),
                    true,
                    false,
                )
                .on_click(cx.listener(move |this, _, window, cx| {
                    if cancel_step {
                        this.cancel_device_step(cx);
                    } else {
                        this.dismiss_menu(window, cx);
                    }
                })),
            )
            .child(
                div()
                    .debug_selector(|| "device-setup-sheet-title".into())
                    .flex_1()
                    .min_w_0()
                    .text_center()
                    .font_weight(FontWeight::SEMIBOLD)
                    .child(fl!("add-device-title")),
            );
        if let Some(setup) = setup
            && let Some(primary) = actions.and_then(|actions| actions.primary)
        {
            header = header.child(self.render_device_primary(
                primary,
                "device-setup-header-primary",
                setup,
                cx,
            ));
        }
        div()
            .debug_selector(|| "device-setup-dialog".into())
            .flex_1()
            .min_h_0()
            .max_h_full()
            .overflow_hidden()
            .flex()
            .flex_col()
            .child(header)
            .child(pane)
    }

    pub(super) fn device_sheet_field(
        &self,
        index: usize,
        label: String,
        field: &Entity<SearchInput>,
        cx: &mut Context<Self>,
    ) -> Div {
        let focus = field.read(cx).focus.clone();
        div()
            .debug_selector(move || format!("device-setup-field-{index}"))
            .flex_1()
            .min_w_0()
            .flex()
            .flex_col()
            .gap(px(6.))
            .child(label)
            .child(
                div()
                    .id(("device-setup-input", index))
                    .debug_selector(move || format!("device-setup-input-{index}"))
                    .flex_none()
                    .h(px(FIELD))
                    .w_full()
                    .min_w_0()
                    .px(px(8.))
                    .flex()
                    .items_center()
                    .rounded(px(crate::config::corners::CONTROL))
                    .border_1()
                    .border_color(rgb(self.theme.active))
                    .bg(rgb(self.theme.background))
                    .cursor(CursorStyle::IBeam)
                    .on_mouse_down(MouseButton::Left, move |_, window, cx| {
                        window.focus(&focus, cx);
                        window.request_virtual_keyboard();
                        cx.stop_propagation();
                    })
                    .child(field.clone()),
            )
    }
}
