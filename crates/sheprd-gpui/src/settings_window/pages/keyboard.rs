//! iPad shortcut presentation, following the selected host's resolved keymap.
// i18n: migrated
use super::{SettingsWindow, ui};
use crate::{
    config::{KeybindingSource, ShortcutStyle, Theme, corners, mix, preferences::Preference},
    controls::Command,
    direction::Logical,
    i18n::{self, fl},
    ipad_ui::{
        self, kbd,
        metrics::{SHORTCUT_ROW, STYLE_ROW, TOUCH},
    },
    keymap::{Keymap, KeymapSource, LocalPrefixSource, ResolvedKeymap, glyphs},
};
use gpui::{prelude::*, *};

const TAB_NUMBERS: &[Command] = &[
    Command::TabNumber(1),
    Command::TabNumber(2),
    Command::TabNumber(3),
    Command::TabNumber(4),
    Command::TabNumber(5),
    Command::TabNumber(6),
    Command::TabNumber(7),
    Command::TabNumber(8),
    Command::TabNumber(9),
];

/// Each displayed action is declared once, in the spec's reading order.
pub(in crate::settings_window) const KEYBOARD_ROWS: &[(&str, &[Command])] = &[
    ("ipad-keyboard-row-new-tab", &[Command::Tab]),
    (
        "ipad-keyboard-row-previous-next-tab",
        &[Command::PreviousTab, Command::NextTab],
    ),
    ("ipad-keyboard-row-tab-number", TAB_NUMBERS),
    (
        "ipad-keyboard-row-split",
        &[Command::SplitRight, Command::SplitDown],
    ),
    ("ipad-keyboard-row-close-pane", &[Command::ClosePane]),
    ("ipad-keyboard-row-zoom", &[Command::Zoom]),
    ("ipad-keyboard-row-copy-mode", &[Command::CopyMode]),
    ("ipad-keyboard-row-spaces", &[Command::WorkspacePicker]),
    ("ipad-keyboard-row-new-space", &[Command::Workspace]),
    ("ipad-keyboard-row-settings", &[Command::Settings]),
    ("ipad-keyboard-row-shortcuts", &[Command::Keybinds]),
    ("ipad-keyboard-row-sidebar", &[Command::ToggleSidebar]),
];

/// Collapse numbered tabs only when every resolved key agrees with the range.
/// Overrides can unbind or move any digit; those values must remain visible.
pub(in crate::settings_window) fn shortcut_values(
    keymap: &Keymap,
    commands: &[Command],
    style: ShortcutStyle,
) -> Vec<String> {
    let values: Vec<_> = commands
        .iter()
        .map(|&command| {
            keymap
                .styled(command, style)
                .map(glyphs)
                .unwrap_or_else(|| "—".into())
        })
        .collect();
    if commands == TAB_NUMBERS
        && let Some(stem) = values.first().and_then(|value| value.strip_suffix('1'))
        && values
            .iter()
            .enumerate()
            .all(|(index, value)| *value == format!("{stem}{}", index + 1))
    {
        return vec![format!("{stem}1–9")];
    }
    if values.iter().all(|value| value == "—") {
        return vec!["—".into()];
    }
    values
}

pub(in crate::settings_window) fn samples(keymap: &Keymap, style: ShortcutStyle) -> Vec<String> {
    [Command::Tab, Command::SplitRight]
        .into_iter()
        .flat_map(|command| shortcut_values(keymap, &[command], style))
        .chain(shortcut_values(keymap, TAB_NUMBERS, style))
        .collect()
}

pub(in crate::settings_window) fn prefix_value(keymap: &Keymap) -> String {
    keymap
        .prefix_label()
        .map(|label| glyphs(&label))
        .unwrap_or_else(|| "—".into())
}

/// An opt-in is insufficient: a missing or rejected profile uses local keys.
#[cfg(test)]
pub(in crate::settings_window) fn prefix_host(owner: &crate::SheprdWindow) -> Option<&str> {
    match owner.resolved_keymap().source {
        KeymapSource::Server(host) => Some(host),
        KeymapSource::Local(_) => None,
    }
}

pub(in crate::settings_window) fn prefix_guidance(source: KeymapSource<'_>) -> (String, String) {
    match source {
        KeymapSource::Server(host) => (
            fl!("ipad-keyboard-prefix-from-host", host = host),
            fl!("ipad-keyboard-prefix-edit-host", host = host),
        ),
        KeymapSource::Local(source) => {
            let origin = match source {
                LocalPrefixSource::Default(_) => fl!("ipad-keyboard-prefix-default"),
                LocalPrefixSource::Configured(_) => fl!("ipad-keyboard-prefix-local"),
            };
            let edit = source.path().map_or_else(
                || fl!("ipad-keyboard-prefix-edit-local"),
                |path| {
                    fl!(
                        "ipad-keyboard-prefix-edit-path",
                        path = path.to_string_lossy().as_ref()
                    )
                },
            );
            (origin, edit)
        }
    }
}

impl SettingsWindow {
    pub(in crate::settings_window) fn render_keyboard(&self, cx: &mut Context<Self>) -> Div {
        if !ipad_ui::layout() {
            return div();
        }
        let source = self.source.upgrade();
        let owner = source.as_ref().map(|source| source.read(cx));
        let resolved = owner.map_or_else(
            || ResolvedKeymap {
                keymap: &self.config.keybindings,
                source: KeymapSource::Local(&self.config.keybinding_prefix_source),
            },
            |owner| owner.resolved_keymap(),
        );
        let keymap = resolved.keymap;
        let theme = &self.theme;
        let endpoint = owner.and_then(|owner| owner.endpoints.get(owner.selected_endpoint));
        let server = endpoint.is_some_and(|endpoint| {
            self.config.keybinding_source(&endpoint.id) == KeybindingSource::Server
        });
        let (prefix_source, prefix_edit) = prefix_guidance(resolved.source);
        let prefix = prefix_value(keymap);
        let enabled = !self.quitting && self.closing.is_none();
        let server_enabled = enabled && self.keyboard_keybinding_choice(cx).is_some();
        let switch = div()
            .id("keyboard-server-toggle")
            .debug_selector(|| "keyboard-server-toggle".into())
            .role(Role::Switch)
            .aria_toggled(if server {
                Toggled::True
            } else {
                Toggled::False
            })
            .size(px(TOUCH))
            .flex_none()
            .row()
            .items_center()
            .justify_center()
            .child(switch_mark(theme, server));
        let switch = crate::accessibility::disabled(switch, !server_enabled)
            .when(!server_enabled, |switch| switch.opacity(0.5))
            .when(server_enabled, |switch| {
                switch
                    .cursor_pointer()
                    .on_click(cx.listener(|this, _, _, cx| {
                        if let Some((profile, source)) = this.keyboard_keybinding_choice(cx) {
                            this.save_preference(
                                Preference::DeviceKeybindings { profile, source },
                                cx,
                            );
                        }
                    }))
            });
        div()
            .flex()
            .flex_col()
            .min_w_0()
            .child(ui::section(
                theme,
                &fl!("ipad-keyboard-style-heading"),
                None,
            ))
            .children(
                ShortcutStyle::ALL
                    .iter()
                    .map(|&style| self.keyboard_style_row(keymap, style, enabled, cx)),
            )
            .child(
                div()
                    .my(px(12.))
                    .text_size(px(12.))
                    .line_height(px(18.))
                    .text_color(rgb(theme.subtext()))
                    .child(fl!("ipad-keyboard-style-description")),
            )
            .child(
                ui::row(theme, "keyboard-prefix", fl!("ipad-keyboard-prefix-key"))
                    .description(prefix_source)
                    .control(
                        kbd(prefix, theme, &self.config.ui)
                            .debug_selector(|| "keyboard-prefix-value".into()),
                    ),
            )
            .child(
                div()
                    .min_w_0()
                    .px(px(12.))
                    .py(px(8.))
                    .text_size(px(12.))
                    .line_height(px(18.))
                    .text_color(rgb(theme.subtext()))
                    .child(prefix_edit),
            )
            .child(
                ui::row(
                    theme,
                    "keyboard-server",
                    fl!("ipad-keyboard-server-keybindings"),
                )
                .description(fl!("ipad-keyboard-server-keybindings-note"))
                .switch(switch),
            )
            .child(ui::section(
                theme,
                &fl!("ipad-keyboard-shortcuts-heading"),
                None,
            ))
            .child(self.keyboard_table(keymap))
    }

    fn keyboard_keybinding_choice(&self, cx: &App) -> Option<(String, KeybindingSource)> {
        let source = self.source.upgrade()?;
        let source = source.read(cx);
        let endpoint = source.endpoints.get(source.selected_endpoint)?;
        self.config.toggle_keybinding_source(&endpoint.id)
    }

    fn keyboard_style_row(
        &self,
        keymap: &Keymap,
        style: ShortcutStyle,
        enabled: bool,
        cx: &Context<Self>,
    ) -> Stateful<Div> {
        let theme = &self.theme;
        let selected = self.config.shortcut_style == style;
        let accent = ipad_ui::accent(theme);
        let id = format!("keyboard-style-{}", style.name());
        let tracked = SharedString::from(id.clone());
        let selector = id.clone();
        let row = div()
            .id(SharedString::from(id))
            .debug_selector(move || selector.clone())
            .role(Role::RadioButton)
            .aria_label(style.label())
            .aria_toggled(if selected {
                Toggled::True
            } else {
                Toggled::False
            })
            .relative()
            .min_h(px(STYLE_ROW))
            .flex_none()
            .row()
            .items_center()
            .gap(px(12.))
            .px(px(12.))
            .rounded(px(corners::CONTROL))
            .border_1()
            .border_color(rgb(if selected { accent } else { theme.active }))
            .when(selected, |row| row.bg(rgb(mix(theme.surface, accent, 22))))
            .child(super::super::search::track_bounds(tracked))
            .child(
                div()
                    .size(px(16.))
                    .flex_none()
                    .row()
                    .items_center()
                    .justify_center()
                    .rounded_full()
                    .border_1()
                    .border_color(rgb(if selected { accent } else { theme.muted }))
                    .when(selected, |mark| {
                        mark.child(div().size(px(8.)).rounded_full().bg(rgb(accent)))
                    }),
            )
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .flex_col()
                    .gap(px(4.))
                    .child(style.label())
                    .child(
                        div().row().flex_wrap().gap(px(4.)).children(
                            samples(keymap, style)
                                .into_iter()
                                .map(|value| kbd(value, theme, &self.config.ui)),
                        ),
                    ),
            )
            .when(enabled && !selected, |row| {
                row.cursor_pointer()
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.save_preference(Preference::ShortcutStyle(style), cx);
                    }))
            });
        crate::accessibility::disabled(row, !enabled)
    }

    fn keyboard_table(&self, keymap: &Keymap) -> Stateful<Div> {
        let theme = &self.theme;
        let selected = self.config.shortcut_style;
        let heading = table_row(theme)
            .child(
                action_cell()
                    .font_weight(FontWeight::SEMIBOLD)
                    .child(fl!("ipad-keyboard-column-action")),
            )
            .children(ShortcutStyle::ALL.iter().map(|&style| {
                style_cell(
                    theme,
                    style == selected,
                    format!("keyboard-column-{}", style.name()),
                )
                .font_weight(FontWeight::SEMIBOLD)
                .child(style.label())
            }));
        div()
            .id("keyboard-shortcuts")
            .debug_selector(|| "keyboard-shortcuts".into())
            .relative()
            .min_w_0()
            .w_full()
            .overflow_x_scroll()
            .child(super::super::search::track_bounds(
                "keyboard-shortcuts".into(),
            ))
            .child(
                div()
                    .min_w(px(650.))
                    .w_full()
                    .flex()
                    .flex_col()
                    .child(heading)
                    .children(KEYBOARD_ROWS.iter().map(|&(message, commands)| {
                        table_row(theme)
                            .debug_selector(move || message.to_owned())
                            .child(action_cell().child(i18n::message(message, &[])))
                            .children(ShortcutStyle::ALL.iter().map(|&style| {
                                style_cell(
                                    theme,
                                    style == selected,
                                    format!("{message}-{}", style.name()),
                                )
                                .children(
                                    shortcut_values(keymap, commands, style)
                                        .into_iter()
                                        .enumerate()
                                        .map(move |(index, value)| {
                                            kbd(value, theme, &self.config.ui).debug_selector(
                                                move || {
                                                    format!("{message}-{}-{index}", style.name())
                                                },
                                            )
                                        }),
                                )
                            }))
                    })),
            )
    }
}

fn switch_mark(theme: &Theme, on: bool) -> Div {
    let fill = if on {
        ipad_ui::accent(theme)
    } else {
        theme.active
    };
    div()
        .w(px(36.))
        .h(px(22.))
        .p(px(2.))
        .flex_none()
        .row()
        .items_center()
        .rounded_full()
        .bg(rgb(fill))
        .when(on, |mark| mark.justify_end())
        .child(
            div()
                .size(px(18.))
                .rounded_full()
                .bg(rgb(theme.text_on(fill))),
        )
}

fn table_row(theme: &Theme) -> Div {
    div()
        .row()
        .h(px(SHORTCUT_ROW))
        .flex_none()
        .border_b_1()
        .border_color(rgb(theme.active))
        .text_size(px(11.))
}

fn action_cell() -> Div {
    div()
        .w(px(230.))
        .flex_none()
        .h_full()
        .row()
        .items_center()
        .px(px(8.))
}

fn style_cell(theme: &Theme, selected: bool, id: String) -> Stateful<Div> {
    div()
        .id(SharedString::from(id.clone()))
        .debug_selector(move || id.clone())
        .flex_1()
        .min_w_0()
        .h_full()
        .row()
        .items_center()
        .gap(px(4.))
        .px(px(6.))
        .overflow_x_scroll()
        .when(selected, |cell| {
            cell.bg(rgb(mix(theme.surface, ipad_ui::accent(theme), 22)))
        })
}
