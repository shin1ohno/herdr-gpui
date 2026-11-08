//! A transient guide over the terminal; it never takes text-input focus.
use super::SheprdWindow;
use crate::{
    config::ShortcutStyle,
    controls::{COMMANDS, Command},
    fonts::StyledFont,
    i18n::fl,
    ipad_ui::kbd,
    keymap::{Keymap, glyphs},
    motion,
};
use gpui::{prelude::*, *};

mod custom;

#[derive(Default)]
pub(crate) struct Guide {
    pub(crate) manual: bool,
    motion: motion::Toggle,
}

const GROUPS: &[(&str, &[Command])] = &[
    (
        "shortcut-guide-tabs",
        &[Command::Tab, Command::PreviousTab, Command::NextTab],
    ),
    (
        "shortcut-guide-panes",
        &[
            Command::SplitRight,
            Command::SplitDown,
            Command::Zoom,
            Command::ClosePane,
            Command::FocusLeft,
            Command::FocusDown,
            Command::FocusUp,
            Command::FocusRight,
        ],
    ),
    (
        "shortcut-guide-navigation",
        &[
            Command::WorkspacePicker,
            Command::ToggleSidebar,
            Command::Palette,
            Command::Settings,
        ],
    ),
];

fn binding(keymap: &Keymap, command: Command, style: ShortcutStyle, armed: bool) -> Option<String> {
    let value = glyphs(keymap.styled(command, style)?);
    if armed {
        Some(value.split_once(' ')?.1.to_owned())
    } else {
        Some(value)
    }
}

impl SheprdWindow {
    pub(crate) fn toggle_shortcut_guide(&mut self, cx: &mut Context<Self>) {
        let open = !(self.ipad.guide.manual || self.prefix_armed);
        self.ipad.guide.manual = open;
        if open {
            self.resize_mode = false;
        }
        self.prefix_armed = open && self.config.shortcut_style == ShortcutStyle::HerdrPrefix;
        if cfg!(target_os = "ios") && std::env::var_os("HERDR_IOS_ESCAPE_DIAGNOSTICS").is_some() {
            tracing::debug!(
                open,
                composing = !self.marked.is_empty(),
                native_trace_enabled = tracing::enabled!(target: "herdr_uikit::window::text_input::keys", tracing::Level::DEBUG),
                "iPad shortcut guide diagnostic"
            );
        }
        cx.notify();
    }

    fn close_shortcut_guide(&mut self, cx: &mut Context<Self>) {
        self.ipad.guide.manual = false;
        self.prefix_armed = false;
        cx.notify();
    }

    pub(crate) fn render_shortcut_guide(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        let open = (self.ipad.guide.manual || self.prefix_armed) && self.menu.page.is_none();
        let now = motion::now();
        self.ipad.guide.motion.set(open, now, cfg!(test));
        if self.ipad.guide.motion.moving(now) {
            window.request_animation_frame();
        }
        let shown = self.ipad.guide.motion.shown(now);
        if shown == 0. {
            return None;
        }
        let armed = self.prefix_armed;
        let style = if armed {
            ShortcutStyle::HerdrPrefix
        } else {
            self.config.shortcut_style
        };
        let title = if armed {
            fl!("shortcut-guide-next")
        } else {
            fl!("status-bar-shortcuts")
        };
        let prefix = self.keymap().prefix_label().map(|key| glyphs(&key));
        let connected = self.live.status.is_connected();
        let core = self.core_terminals.focused_pane(window, cx);
        let pane = core.is_some()
            || self
                .live
                .snapshot
                .as_ref()
                .is_some_and(|snapshot| snapshot.focused_pane_id.is_some());
        let observe = core
            .as_ref()
            .is_some_and(|(tab, _)| !self.core_terminals.drives(*tab));
        let mut sections = Vec::new();
        for &(heading, commands) in GROUPS {
            let mut rows = Vec::new();
            for &command in commands {
                let local = matches!(
                    command,
                    Command::ToggleSidebar
                        | Command::Palette
                        | Command::Settings
                        | Command::WorkspacePicker
                );
                let pane_action = matches!(
                    command,
                    Command::SplitRight
                        | Command::SplitDown
                        | Command::Zoom
                        | Command::ClosePane
                        | Command::FocusLeft
                        | Command::FocusRight
                        | Command::FocusUp
                        | Command::FocusDown
                );
                if pane_action
                    && (!pane
                        || (observe && matches!(command, Command::SplitRight | Command::SplitDown)))
                {
                    continue;
                }
                if !local && !connected && core.is_none() {
                    continue;
                }
                let Some(key) = binding(self.keymap(), command, style, armed) else {
                    continue;
                };
                let Some(info) = COMMANDS.iter().find(|info| info.command == command) else {
                    continue;
                };
                rows.push(
                    div()
                        .id(SharedString::from(format!("shortcut-guide-{command:?}")))
                        .debug_selector(move || format!("shortcut-guide-{command:?}"))
                        .role(Role::Button)
                        .aria_label(crate::i18n::command(info))
                        .h(px(44.))
                        .w_full()
                        .flex_none()
                        .flex()
                        .items_center()
                        .gap(px(12.))
                        .px(px(12.))
                        .rounded(px(8.))
                        .cursor_pointer()
                        .hover(|row| row.bg(rgb(self.theme.active)))
                        .child(
                            div()
                                .min_w(px(70.))
                                .child(kbd(key, &self.theme, &self.config.ui)),
                        )
                        .child(div().min_w_0().truncate().child(crate::i18n::command(info)))
                        .on_click(cx.listener(move |this, _, window, cx| {
                            this.close_shortcut_guide(cx);
                            this.command(command, window, cx);
                        })),
                );
            }
            if rows.is_empty() {
                continue;
            }
            sections.push(
                div()
                    .flex_1()
                    .min_w(px(220.))
                    .flex()
                    .flex_col()
                    .gap(px(4.))
                    .child(
                        div()
                            .px(px(12.))
                            .py(px(8.))
                            .text_color(rgb(self.theme.subtext()))
                            .child(crate::i18n::message(heading, &[])),
                    )
                    .children(rows),
            );
        }
        sections.extend(self.custom_guide_section(style, armed, cx));
        let card = div()
            .id("shortcut-guide")
            .debug_selector(|| "shortcut-guide".into())
            .role(Role::Group)
            .aria_label(title.clone())
            .w_full()
            .max_w(px(900.))
            .max_h(relative(0.8))
            .min_h_0()
            .flex()
            .flex_col()
            .rounded(px(16.))
            .border_1()
            .border_color(rgb(self.theme.active))
            .bg(rgb(self.theme.surface))
            .text_font(&self.config.ui)
            .text_size(px(14.))
            .text_color(rgb(self.theme.foreground))
            .overflow_hidden()
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .items_center()
                    .gap(px(12.))
                    .px(px(20.))
                    .py(px(16.))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .when(self.ipad_compact(), |title| title.w_full().flex_none())
                            .child(title),
                    )
                    .children(
                        prefix
                            .filter(|_| armed)
                            .map(|key| kbd(key, &self.theme, &self.config.ui)),
                    )
                    .child(
                        div()
                            .text_color(rgb(self.theme.subtext()))
                            .child(style.label()),
                    )
                    .child(
                        div()
                            .id("shortcut-guide-close")
                            .debug_selector(|| "shortcut-guide-close".into())
                            .role(Role::Button)
                            .aria_label(fl!("shortcut-guide-close"))
                            .min_w(px(44.))
                            .h(px(44.))
                            .flex()
                            .items_center()
                            .justify_center()
                            .cursor_pointer()
                            .child("×")
                            .on_click(cx.listener(|this, _, _, cx| this.close_shortcut_guide(cx))),
                    ),
            )
            .child(
                div()
                    .id("shortcut-guide-rows")
                    .min_h_0()
                    .overflow_y_scroll()
                    .flex()
                    .flex_wrap()
                    .p(px(8.))
                    .children(sections),
            )
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .items_center()
                    .gap(px(12.))
                    .px(px(20.))
                    .py(px(12.))
                    .border_t_1()
                    .border_color(rgb(self.theme.active))
                    .child(
                        div()
                            .flex_1()
                            .text_size(px(12.))
                            .text_color(rgb(self.theme.subtext()))
                            .child(fl!("shortcut-guide-dismiss")),
                    )
                    .child(
                        div()
                            .id("shortcut-guide-reference")
                            .role(Role::Button)
                            .aria_label(fl!("shortcut-guide-reference"))
                            .h(px(44.))
                            .flex()
                            .items_center()
                            .cursor_pointer()
                            .child(fl!("shortcut-guide-reference"))
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.close_shortcut_guide(cx);
                                this.open_keybinds(window, cx);
                            })),
                    )
                    .child(
                        div()
                            .id("shortcut-guide-settings")
                            .debug_selector(|| "shortcut-guide-settings".into())
                            .role(Role::Button)
                            .aria_label(fl!("shortcut-guide-settings"))
                            .h(px(44.))
                            .flex()
                            .items_center()
                            .cursor_pointer()
                            .child(fl!("shortcut-guide-settings"))
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.close_shortcut_guide(cx);
                                crate::settings_window::open_at(
                                    cx.weak_entity(),
                                    Some(crate::settings_window::Section::Keyboard),
                                    cx,
                                );
                            })),
                    ),
            );
        Some(
            div()
                .absolute()
                .inset_0()
                .flex()
                .items_end()
                .justify_center()
                .p(px(16.))
                .pb(px(56.))
                .when(open, |layer| {
                    layer.on_mouse_down(
                        MouseButton::Left,
                        cx.listener(|this, _, _, cx| {
                            this.close_shortcut_guide(cx);
                            cx.stop_propagation();
                        }),
                    )
                })
                .opacity(shown)
                .child(card)
                .into_any_element(),
        )
    }
}

#[cfg(test)]
mod tests;
