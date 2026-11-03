//! iPad connection context and a legend drawn from the resolved shortcut style.
use super::*;
use crate::{
    config::ShortcutStyle,
    controls::Command,
    fonts::StyledFont,
    ipad_ui::{kbd, metrics::TOUCH},
    window::tab_strip::text_width,
};

pub(super) const STATUS_BAR: f32 = 44.;

const HOST_FLOOR: f32 = 96.;
// Status dot 8 + gap 8 + HOST_FLOOR + gap 8 + STATUS_GLYPH.
const HOST_GROUP_MIN: f32 = 8. + 8. + HOST_FLOOR + 8. + STATUS_GLYPH;

impl HerdrWindow {
    pub(super) fn render_ipad_status_bar(&self, cx: &mut Context<Self>) -> Stateful<Div> {
        let endpoint = &self.endpoints[self.selected_endpoint];
        let connected = self.displayed_connection_status(endpoint).is_connected();
        let generation = match endpoint.backend_generation() {
            herdr_backend::Generation::Herdr1 => 1,
            herdr_backend::Generation::Herdr2 => 2,
        };
        let host = fl!(
            "ipad-chrome-status-host",
            host = endpoint.label.as_str(),
            generation = generation
        );
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
                    .min_w(px(HOST_GROUP_MIN))
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
                    .child(HostLabel {
                        text: host.into(),
                        view: cx.entity(),
                        yield_to_status: status.is_some() || prefix.is_some(),
                    })
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
            .children(self.ipad_rail_hint(cx))
            .children(self.ipad_keyboard_button(cx))
    }

    fn ipad_rail_hint(&self, cx: &mut Context<Self>) -> Option<Stateful<Div>> {
        let ring = rgb(crate::ipad_ui::accent(&self.theme));
        (self.sidebar_mode() == crate::sidebar::SidebarMode::Rail).then(|| {
            self.chrome_stop(
                crate::window::chrome_focus::Region::StatusBar,
                div()
                    .id("ipad-rail-hint")
                    .debug_selector(|| "ipad-rail-hint".into())
                    .role(Role::Button)
                    .aria_label(fl!("ipad-rail-expand")),
            )
            .flex()
            .items_center()
            .gap(px(5.))
            .h(px(TOUCH))
            .min_w(px(TOUCH))
            .flex_shrink_1()
            .overflow_hidden()
            .cursor_pointer()
            // Keep the shared tab-stop policy with the iPad accent for focus.
            .focus_visible(move |style| {
                style.shadow(vec![BoxShadow {
                    color: ring.into(),
                    offset: point(px(0.), px(0.)),
                    blur_radius: px(0.),
                    spread_radius: px(2.),
                    inset: true,
                }])
            })
            .children(self.shortcut_hint(Command::ToggleSidebar).map(|hint| {
                kbd(hint, &self.theme, &self.config.ui)
                    .debug_selector(|| "ipad-rail-hint-shortcut".into())
            }))
            .child(div().min_w_0().truncate().child(fl!("ipad-rail-expand")))
            .on_click(
                cx.listener(|this, _, window, cx| this.command(Command::ToggleSidebar, window, cx)),
            )
        })
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
        .flex_wrap()
        .content_start()
        .items_center()
        .h_full()
        .min_w_0()
        .flex_shrink_1()
        .overflow_hidden()
        .whitespace_nowrap()
        .cursor_pointer()
        // An oversized first pair must wrap too, so an empty item holds the
        // full-height first line. Pairs space themselves with a leading
        // margin rather than a gap: a gap after the empty item, or a negative
        // margin cancelling it, would widen the legend's intrinsic width.
        .child(div().w(px(0.)).h(px(STATUS_BAR)).flex_none())
        .children(
            actions
                .into_iter()
                .enumerate()
                .map(|(index, (command, label))| {
                    div()
                        .debug_selector(move || format!("ipad-legend-{command:?}"))
                        .when(index > 0, |pair| pair.ml(px(12.)))
                        .h(px(STATUS_BAR))
                        .flex_none()
                        .flex()
                        .items_center()
                        .gap(px(5.))
                        .children(self.shortcut_hint(command).map(|hint| {
                            kbd(hint, &self.theme, &self.config.ui)
                                .debug_selector(move || format!("ipad-legend-shortcut-{command:?}"))
                        }))
                        .child(label)
                }),
        )
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
struct HostLabel {
    text: SharedString,
    view: Entity<HerdrWindow>,
    yield_to_status: bool,
}

impl RenderOnce for HostLabel {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let view = self.view.read(cx);
        let minimum = if self.yield_to_status {
            0.
        } else {
            text_width(&self.text, &view.config.ui, window).min(HOST_FLOOR)
        };
        div()
            .debug_selector(|| "ipad-status-host".into())
            .min_w(px(minimum))
            .flex_shrink_1()
            .truncate()
            .child(self.text)
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
                .min_w(px(STATUS_GLYPH))
                .flex_shrink_1()
                .overflow_hidden()
                .flex()
                .items_center()
                .gap(px(5.))
                .whitespace_nowrap()
                .child(glyph("icons/keyboard.svg", view.theme.subtext()))
                .child(
                    div()
                        .min_w_0()
                        .flex_shrink_1()
                        .truncate()
                        .child(fl!("ipad-chrome-status-hardware-keyboard")),
                )
        })
    }
}
