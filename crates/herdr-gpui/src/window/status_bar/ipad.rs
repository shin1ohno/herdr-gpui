//! iPad connection context and a legend drawn from the resolved shortcut style.
use super::*;
use crate::{
    config::ShortcutStyle,
    controls::Command,
    fonts::StyledFont,
    ipad_ui::{kbd, metrics::TOUCH},
};

pub(super) const STATUS_BAR: f32 = 44.;

impl HerdrWindow {
    pub(super) fn render_ipad_status_bar(&self, cx: &mut Context<Self>) -> Stateful<Div> {
        let endpoint = &self.endpoints[self.selected_endpoint];
        let connected = self.displayed_connection_status(endpoint).is_connected();
        let generation = match endpoint.backend_generation() {
            herdr_backend::Generation::Herdr1 => 1,
            herdr_backend::Generation::Herdr2 => 2,
        };
        let status = (!connected || self.local_error.is_some() || self.live.error.is_some())
            .then(|| self.selected_connection_text());
        let prefix = self
            .prefix_armed
            .then(|| self.keymap().prefix_label())
            .flatten();
        div()
            .id("connection-status")
            .debug_selector(|| "connection-status".into())
            .role(Role::Status)
            .aria_label(fl!("status-bar-label"))
            .relative()
            .children(self.status_bar_probe())
            .flex()
            .flex_none()
            .h(px(STATUS_BAR))
            .w_full()
            .items_center()
            .gap(px(8.))
            .px(px(12.))
            .overflow_hidden()
            .bg(self.fill().panel(self.theme.surface))
            .text_font(&self.config.ui)
            .text_size(px(self.config.ui.size))
            .text_color(rgb(self.theme.foreground))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .flex_1()
                    .min_w_0()
                    .overflow_hidden()
                    .child(
                        div()
                            .debug_selector(|| "ipad-status-dot".into())
                            .size(px(8.))
                            .flex_none()
                            .rounded_full()
                            .border_1()
                            .border_color(rgb(if connected {
                                self.theme.ink(self.theme.palette[2])
                            } else {
                                self.theme.subtext()
                            }))
                            .when(connected, |dot| {
                                dot.bg(rgb(self.theme.ink(self.theme.palette[2])))
                            }),
                    )
                    .child(
                        div()
                            .debug_selector(|| "ipad-status-host".into())
                            .min_w_0()
                            .flex_shrink_1()
                            .truncate()
                            .child(fl!(
                                "ipad-chrome-status-host",
                                host = endpoint.label.as_str(),
                                generation = generation
                            )),
                    )
                    .child(Hardware { view: cx.entity() })
                    .children(prefix.map(|prefix| {
                        kbd(prefix, &self.theme, &self.config.ui)
                            .debug_selector(|| "prefix-armed".into())
                    }))
                    .children(status.map(|status| {
                        div()
                            .id("connection-message")
                            .debug_selector(|| "connection-message".into())
                            .role(Role::Label)
                            .aria_value(status.clone())
                            .min_w_0()
                            .flex_shrink_1()
                            .truncate()
                            .child(status)
                    })),
            )
            .child(self.ipad_legend(cx))
            .children(self.ipad_keyboard_button(cx))
    }

    fn ipad_legend(&self, cx: &mut Context<Self>) -> Stateful<Div> {
        let actions = match self.config.shortcut_style {
            ShortcutStyle::Ipad => [
                (Command::Tab, fl!("ipad-chrome-legend-tab")),
                (Command::SplitRight, fl!("ipad-chrome-legend-split")),
                (Command::ToggleSidebar, fl!("ipad-chrome-legend-sidebar")),
                (Command::Palette, fl!("ipad-chrome-legend-commands")),
            ],
            ShortcutStyle::HerdrPrefix => [
                (Command::Tab, fl!("ipad-chrome-legend-tab")),
                (Command::SplitRight, fl!("ipad-chrome-legend-split")),
                (Command::ClosePane, fl!("ipad-chrome-legend-close-pane")),
                (Command::Keybinds, fl!("ipad-chrome-legend-help")),
            ],
        };
        self.chrome_stop(
            crate::window::chrome_focus::Region::StatusBar,
            div()
                .id("status-keybinds")
                .debug_selector(|| "status-keybinds".into())
                .role(Role::Button)
                .aria_label(fl!("status-bar-shortcuts")),
        )
        .flex()
        .items_center()
        .gap(px(12.))
        .h_full()
        .min_w_0()
        .flex_shrink_1()
        .overflow_hidden()
        .whitespace_nowrap()
        .cursor_pointer()
        .children(actions.into_iter().map(|(command, label)| {
            div()
                .debug_selector(move || format!("ipad-legend-{command:?}"))
                .flex_none()
                .flex()
                .items_center()
                .gap(px(5.))
                .children(self.shortcut_hint(command).map(|hint| {
                    kbd(hint, &self.theme, &self.config.ui)
                        .debug_selector(move || format!("ipad-legend-shortcut-{command:?}"))
                }))
                .child(label)
        }))
        .on_click(cx.listener(|this, _, window, cx| this.command(Command::Keybinds, window, cx)))
    }

    pub(super) fn ipad_keyboard_button(&self, cx: &mut Context<Self>) -> Option<Stateful<Div>> {
        crate::ipad_ui::active().then(|| {
            self.chrome_stop(
                crate::window::chrome_focus::Region::StatusBar,
                div()
                    .id("status-keyboard")
                    .debug_selector(|| "status-keyboard".into())
                    .role(Role::Button)
                    .aria_label(fl!("ipad-chrome-status-show-keyboard")),
            )
            .size(px(if crate::ipad_ui::layout() {
                TOUCH
            } else {
                STATUS_GLYPH
            }))
            .flex_none()
            .flex()
            .items_center()
            .justify_center()
            .cursor_pointer()
            .child(glyph("icons/keyboard.svg", self.theme.foreground))
            .on_click(cx.listener(|this, _, window, cx| {
                if !this.request_core_terminal_keyboard(window, cx) {
                    window.focus(&this.focus, cx);
                    this.request_terminal_keyboard(window);
                }
            }))
        })
    }
}

#[derive(IntoElement)]
struct Hardware {
    view: Entity<HerdrWindow>,
}

impl RenderOnce for Hardware {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let view = self.view.read(cx);
        div().when(view.ipad.keyboard.hardware(window), |row| {
            row.debug_selector(|| "ipad-status-hardware".into())
                .flex_none()
                .flex()
                .items_center()
                .gap(px(5.))
                .whitespace_nowrap()
                .child(glyph("icons/keyboard.svg", view.theme.subtext()))
                .child(fl!("ipad-chrome-status-hardware-keyboard"))
        })
    }
}
