//! The collapsed sidebar's compact rail, as Herdr's default
//! `ui.sidebar_collapsed_mode = "compact"` draws it: per host a small header
//! box, then a status mark per workspace and per agent. The header box stays
//! its own shape and wears the workspace badge's chrome: the same fill,
//! stroke, and corner as the selected workspace number, one step quieter on
//! every other host. Workspace badges and agent rows share one borderless
//! grid, centered on that box, and the selection highlight is the only other
//! box: one width, wrapped around the glyph, the icon, and the dot. Every
//! mark navigates where its expanded row would, and its tooltip names what
//! the narrow column cannot.

mod ipad;
mod marks;
use marks::*;

use super::{
    agent_name,
    agents::{Indicators, agent_place, state_label, status_indicator, status_text},
    cell::RowState,
    glyph_width, label_text,
    layout::{self, Highlight, SidebarLook},
    line_height, visible_workspace_entries,
    wash::HostMark,
    workspaces::workspace_label,
};
use crate::direction::Logical;
use crate::i18n::fl;
use crate::menu::{ContextMenuEvent, context_menu};
use crate::notification_mutes::{Daemon, Silenced};
use crate::{
    Command, HerdrWindow, NavigationTarget,
    config::{FontConfig, Theme},
    fonts::StyledFont,
    herdr_settings::SidebarCollapsedMode,
};
use gpui::{prelude::*, *};
use herdr_client::protocol::{AgentStatus, ClientShellAgent};
use std::collections::HashMap;

/// Wide enough for the shared mark grid: a two-digit workspace number, an
/// agent icon, and a drawn status icon, plus even padding inside the highlight.
pub(crate) const RAIL_WIDTH: f32 = 64.;
const HOST_MARK: f32 = 22.;
const HOST_BORDER: f32 = 1.;
const HOST_DOT: f32 = 6.;
/// Sidebar-colored ring around the status dot, so the header stroke cannot
/// cut through it.
const HOST_DOT_RING: f32 = 2.;
/// Room between the header stroke and that ring. The ring hangs outside the
/// box; the gap is the sidebar, and the stroke never meets the dot.
const HOST_DOT_GAP: f32 = 2.;
const AGENT_ICON: f32 = 14.;
const MARK_GAP: f32 = 4.;
/// Padding the mark highlight keeps around the shared column grid.
const RAIL_PAD: f32 = 4.;

/// What the sidebar column shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SidebarMode {
    Expanded,
    Rail,
    Hidden,
}

impl SidebarMode {
    /// The column's width, under the same terminal reserve the expanded
    /// sidebar keeps, so a narrow window never loses its terminal to it.
    pub(crate) fn width(self, preferred: Option<f32>, window_width: f32) -> Option<f32> {
        match self {
            Self::Expanded => Some(super::sidebar_width(preferred, window_width)),
            // A window too narrow for any rail gets none rather than a border.
            Self::Rail => {
                let width = if crate::ipad_ui::layout() {
                    crate::ipad_ui::metrics::RAIL
                } else {
                    RAIL_WIDTH
                };
                Some(width.min(window_width - 240.)).filter(|width| *width > 0.)
            }
            Self::Hidden => None,
        }
    }
}

impl HerdrWindow {
    /// Zen mode hides the column whatever the toggle says; otherwise it is
    /// [`Self::chrome_sidebar_mode`].
    pub(crate) fn sidebar_mode(&self) -> SidebarMode {
        if self.zen_layout() {
            return SidebarMode::Hidden;
        }
        self.chrome_sidebar_mode()
    }

    /// The column outside zen mode, which zen mode slides away. Collapsing
    /// follows the local daemon's `ui.sidebar_collapsed_mode`, also while an
    /// SSH host is shown, as Herdr keeps presentation local. Before shared
    /// settings load, Herdr's default rail applies.
    pub(crate) fn chrome_sidebar_mode(&self) -> SidebarMode {
        if self.sidebar_visible || self.sidebar_slide.closing() {
            return SidebarMode::Expanded;
        }
        match self
            .settings
            .shared
            .as_ref()
            .map_or(SidebarCollapsedMode::Compact, |shared| {
                shared.sidebar_collapsed_mode
            }) {
            SidebarCollapsedMode::Compact => SidebarMode::Rail,
            SidebarCollapsedMode::Hidden => SidebarMode::Hidden,
        }
    }

    pub(crate) fn toggle_sidebar(&mut self) {
        self.sidebar_visible = !self.sidebar_visible;
        self.sidebar_slide
            .set(self.sidebar_visible, crate::motion::now());
        self.sidebar_start_pending = false;
    }

    /// Applies `ui.sidebar_start_collapsed` once, from the first shared
    /// settings to load. Later reloads, and a toggle made before the first
    /// load arrives, leave the user's choice alone, as Herdr reads the setting
    /// at startup only.
    pub(crate) fn apply_sidebar_start(&mut self) {
        if !std::mem::take(&mut self.sidebar_start_pending) {
            return;
        }
        if self
            .settings
            .shared
            .as_ref()
            .is_some_and(|shared| shared.sidebar_start_collapsed)
        {
            self.sidebar_visible = false;
        }
    }

    pub(crate) fn render_rail(
        &self,
        indicators: Indicators,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        let width = SidebarMode::Rail
            .width(None, f32::from(window.viewport_size().width))
            .unwrap_or(0.);
        if crate::ipad_ui::layout() {
            return self.render_ipad_rail(width, indicators, cx);
        }
        let look = layout::for_config(&self.config, self.fill());
        let theme = &self.theme;
        let hint = Hint::new(theme);
        let multi = self.endpoints.len() > 1;
        // What each listed host's mutes silence. A muted workspace shows its
        // mark in the icon column, which the rail then keeps for every row.
        let mutes = crate::notification_mutes::Mutes::any(cx);
        let silenced: HashMap<usize, Silenced<'_>> = self
            .displayed_devices()
            .into_iter()
            .filter_map(|(index, endpoint)| {
                let live = if index == self.selected_endpoint {
                    &self.live
                } else {
                    &endpoint.live
                };
                let snapshot = live.snapshot.as_deref()?;
                Some((index, mutes?.silenced(&Daemon::of(endpoint), snapshot)))
            })
            .collect();
        let slot = multi || silenced.values().any(Silenced::any_workspace);
        // A large sidebar font would make the columns wider than the rail and
        // clip the status icons, so the rail steps the font down until they fit.
        let fitted = rail_font(&self.config.sidebar, indicators, multi, slot);
        let font = &fitted;
        let columns = rail_columns(font, indicators, multi, slot);
        let split = self.sidebar_split.unwrap_or(0.5).clamp(0.1, 0.9);
        let mut spaces = div()
            .id("rail-spaces")
            .debug_selector(|| "rail-spaces".into())
            .flex()
            .flex_col()
            .flex_1()
            .min_h_0()
            .overflow_y_scroll();
        let mut agents = div()
            .id("rail-agents")
            .debug_selector(|| "rail-agents".into())
            .flex()
            .flex_col()
            .flex_1()
            .min_h_0()
            .overflow_y_scroll();
        for (endpoint_index, endpoint) in self.displayed_devices() {
            if !self.device_visible(&endpoint.id) {
                continue;
            }
            let selected = endpoint_index == self.selected_endpoint;
            let endpoint_id = endpoint.id.clone();
            let initial = host_initial(&endpoint.label);
            if multi {
                let select_id = endpoint_id.clone();
                let menu_id = endpoint_id.clone();
                let connected = endpoint.live.status.is_connected();
                let text = if endpoint.collapsed {
                    fl!("sidebar-rail-collapsed", host = endpoint.label.as_str())
                } else {
                    endpoint.label.clone()
                };
                let box_id = endpoint_id.clone();
                let chrome = host_chrome(selected, look, theme);
                spaces = spaces.child(
                    // No row highlight here. The letter's own box is the
                    // header; the selection highlight belongs to the rows
                    // beneath it, and painting both made two boxes clash.
                    host_header(font, look)
                        .child(
                            div()
                                .debug_selector(move || format!("rail-host-box-{box_id}"))
                                .relative()
                                .size(px(HOST_MARK))
                                .flex_none()
                                .row()
                                .items_center()
                                .justify_center()
                                .rounded(px(chrome.radius))
                                .border_1()
                                .border_color(chrome.border)
                                .bg(chrome.fill)
                                .text_color(chrome.letter)
                                .child(label_text(&initial))
                                .child(host_status(
                                    format!("rail-host-status-{endpoint_id}"),
                                    if connected {
                                        crate::menu::online(theme)
                                    } else {
                                        theme.muted
                                    },
                                    theme.sidebar_background(),
                                    width,
                                )),
                        )
                        .id(SharedString::from(format!("rail-host-{endpoint_id}")))
                        .debug_selector(|| format!("rail-host-{endpoint_id}"))
                        .cursor_pointer()
                        .tooltip(hint.with(text))
                        .map(|row| {
                            context_menu(
                                row,
                                cx.listener(move |this, event: &ContextMenuEvent, window, cx| {
                                    cx.stop_propagation();
                                    this.open_host_menu(&menu_id, event.position, window, cx);
                                    this.menu.opening_right_click = event.mouse.is_some()
                                        && this.menu.page == Some(crate::menu::Page::Host);
                                }),
                            )
                        })
                        .on_click(cx.listener(move |this, _, window, cx| {
                            this.select_endpoint(&select_id, cx);
                            window.focus(&this.focus, cx);
                        })),
                );
            }
            if multi && endpoint.collapsed {
                continue;
            }
            let live = if selected { &self.live } else { &endpoint.live };
            let Some(snapshot) = &live.snapshot else {
                continue;
            };
            let collapsed_repos = if endpoint_index == 0 {
                &self.collapsed_repos
            } else {
                &endpoint.collapsed_repos
            };
            let host = multi.then_some(endpoint.label.as_str());
            for (index, indented, _) in
                visible_workspace_entries(&snapshot.workspaces, collapsed_repos)
            {
                let workspace = &snapshot.workspaces[index];
                let id = workspace.workspace_id.clone();
                let key = format!("rail-workspace-{endpoint_id}-{id}");
                let navigate_endpoint = endpoint_id.clone();
                let context_endpoint = endpoint_id.clone();
                let context_id = id.clone();
                let focused = selected && workspace.focused;
                let number = workspace.number.to_string();
                let muted = silenced
                    .get(&endpoint_index)
                    .is_some_and(|silenced| silenced.workspace(&workspace.workspace_id));
                let marks = div()
                    .row()
                    .items_center()
                    .gap(px(columns.gap))
                    .child(mark_slot(
                        format!("rail-mark-{key}"),
                        columns.badge,
                        columns.height,
                        if focused {
                            theme.foreground
                        } else {
                            theme.muted
                        },
                        &number,
                    ))
                    .when_some(columns.icon, |marks, icon| {
                        marks.child(
                            div()
                                .flex_none()
                                .w(px(icon))
                                .h(px(columns.height))
                                .flex()
                                .items_center()
                                .justify_center()
                                .when(muted, |slot| {
                                    // The rail's rows name nothing for a screen
                                    // reader, so the mark names itself.
                                    slot.debug_selector(|| format!("rail-muted-{key}")).child(
                                        crate::icons::spoken_muted(
                                            SharedString::from(format!("rail-muted-mark-{key}")),
                                            crate::icons::icon(
                                                crate::icons::MUTED,
                                                icon - 2.,
                                                theme.subtext(),
                                            ),
                                        ),
                                    )
                                }),
                        )
                    })
                    .child(dot_slot(
                        format!("rail-dot-{key}"),
                        columns.dot,
                        columns.height,
                        indicator(workspace.agent_status, font, indicators),
                    ));
                spaces = spaces.child(
                    aligned_row(
                        &key,
                        RowState {
                            selected: focused,
                            ..RowState::default()
                        },
                        font,
                        look,
                        theme,
                        columns,
                        marks,
                    )
                    .id(SharedString::from(key.clone()))
                    .debug_selector(|| key)
                    .cursor_pointer()
                    .tooltip(hint.with(describe(
                        host,
                        workspace_label(workspace, indented),
                        None,
                        muted_word(status_word(workspace.agent_status), muted).as_deref(),
                    )))
                    .map(|row| {
                        context_menu(
                            row,
                            cx.listener(move |this, event: &ContextMenuEvent, window, cx| {
                                cx.stop_propagation();
                                if this.navigate_endpoint(
                                    &context_endpoint,
                                    NavigationTarget::Workspace(&context_id),
                                    cx,
                                ) {
                                    this.open_workspace_menu(
                                        &context_id,
                                        event.position,
                                        window,
                                        cx,
                                    );
                                    this.menu.opening_right_click = event.mouse.is_some()
                                        && this.menu.page == Some(crate::menu::Page::Workspace);
                                }
                            }),
                        )
                    })
                    .on_click(cx.listener(move |this, _, window, cx| {
                        this.navigate_endpoint(
                            &navigate_endpoint,
                            NavigationTarget::Workspace(&id),
                            cx,
                        );
                        window.focus(&this.focus, cx);
                    })),
                );
            }
        }
        // Agents come after every host: the panel interleaves them.
        let panel = if self.config.show_agents {
            self.panel_agents()
        } else {
            Vec::new()
        };
        for (index, agent) in panel {
            let Some(endpoint) = self.endpoints.get(index) else {
                continue;
            };
            // A collapsed host folds its agents away with its workspaces.
            if multi && endpoint.collapsed {
                continue;
            }
            let selected = index == self.selected_endpoint;
            let live = if selected { &self.live } else { &endpoint.live };
            let Some(snapshot) = live.snapshot.as_deref() else {
                continue;
            };
            let endpoint_id = &endpoint.id;
            let host = multi.then_some(endpoint.label.as_str());
            let id = agent.pane_id.clone();
            let key = format!("rail-agent-{endpoint_id}-{id}");
            let navigate_endpoint = endpoint_id.clone();
            let menu_endpoint = endpoint_id.clone();
            let menu_pane = id.clone();
            let focused = selected && agent.focused;
            let place = agent_place(agent, snapshot);
            let place = place.map(|(workspace, tab)| match tab {
                Some(tab) => format!("{workspace} / {tab}"),
                None => workspace.to_owned(),
            });
            let icon = AGENT_ICON.min(font.size + 1.);
            let icon_color = if focused {
                theme.foreground
            } else {
                theme.muted
            };
            let icon_path = crate::icons::of_agent(agent).path();
            // The host letter shares the workspace number's column. With one
            // host there is no letter, so the icon takes that column and still
            // starts on the badge's left edge.
            let badge = if multi {
                mark_slot(
                    format!("rail-mark-{key}"),
                    columns.badge,
                    columns.height,
                    theme.muted,
                    &host_initial(&endpoint.label),
                )
            } else {
                div()
                    .debug_selector(|| format!("rail-mark-{key}"))
                    .flex_none()
                    .w(px(columns.badge))
                    .h(px(columns.height))
                    .row()
                    .items_center()
                    .child(agent_icon(
                        format!("rail-agent-icon-{endpoint_id}-{id}"),
                        icon,
                        icon_color,
                        icon_path,
                    ))
            };
            let marks = div()
                .row()
                .items_center()
                .gap(px(columns.gap))
                .child(badge)
                .when(multi, |marks| {
                    marks.child(agent_icon(
                        format!("rail-agent-icon-{endpoint_id}-{id}"),
                        icon,
                        icon_color,
                        icon_path,
                    ))
                })
                // One host whose muted workspaces took the icon column.
                .when_some(columns.icon.filter(|_| !multi), |marks, icon| {
                    marks.child(div().flex_none().w(px(icon)).h(px(columns.height)))
                })
                .child(dot_slot(
                    format!("rail-dot-{key}"),
                    columns.dot,
                    columns.height,
                    indicator(agent.agent_status, font, indicators),
                ));
            agents = agents.child(
                aligned_row(
                    &key,
                    RowState {
                        selected: focused,
                        ..RowState::default()
                    },
                    font,
                    look,
                    theme,
                    columns,
                    marks,
                )
                .id(SharedString::from(key.clone()))
                .debug_selector(|| key)
                .cursor_pointer()
                .tooltip(hint.with(describe(
                    host,
                    agent_name(agent),
                    place.as_deref(),
                    agent_status_word(agent).as_deref(),
                )))
                // The expanded sidebar's agent rows have long had this menu;
                // rail rows gain it on iPad, where a long press is the only
                // way to reach it, and stay as they were on the desktop.
                .when(crate::ipad_ui::active(), |row| {
                    context_menu(
                        row,
                        cx.listener(move |this, event: &ContextMenuEvent, window, cx| {
                            cx.stop_propagation();
                            this.open_agent_menu(
                                &menu_endpoint,
                                &menu_pane,
                                event.position,
                                window,
                                cx,
                            );
                            this.menu.opening_right_click &= event.mouse.is_some();
                        }),
                    )
                })
                .on_click(cx.listener(move |this, _, window, cx| {
                    this.navigate_endpoint(&navigate_endpoint, NavigationTarget::Pane(&id), cx);
                    window.focus(&this.focus, cx);
                })),
            );
        }
        div()
            .id("sidebar-rail")
            .debug_selector(|| "sidebar-rail".into())
            .relative()
            .w(px(width))
            .flex_none()
            .h_full()
            .min_h_0()
            .overflow_hidden()
            .flex()
            .flex_col()
            .text_font(font)
            .text_size(px(font.size))
            .line_height(px(line_height(font)))
            .text_color(rgb(theme.foreground))
            .bg(self.fill().panel(theme.sidebar_background()))
            .border_e(px(1.))
            .border_color(rgb(theme.active))
            // No expand control of its own: the sidebar toggle is always on
            // screen, over the rail beside the activity bar, else in the
            // header or leading the tab row.
            .child(
                div()
                    .debug_selector(|| "rail-spaces-section".into())
                    .flex()
                    .flex_col()
                    .flex_1()
                    .map(|mut section| {
                        section.style().flex_grow =
                            Some(if self.config.show_agents { split } else { 1. });
                        section
                    })
                    .min_h_0()
                    .overflow_hidden()
                    .child(spaces),
            )
            .when(self.config.show_agents, |rail| {
                rail.child(div().flex_none().h(px(1.)).bg(rgb(theme.active)))
                    .child(
                        div()
                            .debug_selector(|| "rail-agents-section".into())
                            .flex()
                            .flex_col()
                            .flex_1()
                            .map(|mut section| {
                                section.style().flex_grow = Some(1. - split);
                                section
                            })
                            .min_h_0()
                            .overflow_hidden()
                            .child(agents),
                    )
            })
            .child(
                rail_button("rail-new-workspace", font, look, theme)
                    .text_color(rgb(theme.muted))
                    .hover(|s| s.text_color(rgb(theme.foreground)))
                    .tooltip(hint.with(fl!("sidebar-new-workspace")))
                    .on_click(cx.listener(|this, _, window, cx| {
                        this.command(Command::Workspace, window, cx);
                    }))
                    .child(label_text("+")),
            )
    }
}

/// The host's initial, as Herdr marks a machine in its rail.
pub(super) fn host_initial(label: &str) -> String {
    label
        .trim()
        .chars()
        .next()
        .map_or_else(|| "?".into(), |initial| initial.to_uppercase().collect())
}

/// The status word a tooltip names, none for an unknown status.
fn status_word(status: AgentStatus) -> Option<String> {
    (status != AgentStatus::Unknown).then(|| status_text(status))
}

/// An agent's status word: its integration's state label when it set one.
fn agent_status_word(agent: &ClientShellAgent) -> Option<String> {
    let fallback = status_word(agent.agent_status).unwrap_or_default();
    let word = state_label(agent, &fallback).into_owned();
    (!word.is_empty()).then_some(word)
}

/// A workspace's status word, saying so when its notifications are muted.
fn muted_word(status: Option<String>, muted: bool) -> Option<String> {
    let muted = muted.then(|| fl!("sidebar-spoken-muted"));
    match (status, muted) {
        (Some(status), Some(muted)) => Some([status, muted].join(&fl!("sidebar-spoken-separator"))),
        (status, muted) => status.or(muted),
    }
}

/// A rail mark's tooltip: what the expanded row would have spelled out.
/// `status` is the word for the mark's status: an agent's own state label
/// when its integration set one.
pub(super) fn describe(
    host: Option<&str>,
    name: &str,
    place: Option<&str>,
    status: Option<&str>,
) -> SharedString {
    let mut text = match place {
        Some(place) => fl!("sidebar-rail-in", name = name, place = place),
        None => name.to_owned(),
    };
    if let Some(status) = status {
        text = fl!("sidebar-rail-status", text = text, status = status);
    }
    if let Some(host) = host {
        text = fl!("sidebar-rail-host", host = host, text = text);
    }
    text.into()
}

#[derive(Clone, Copy)]
struct Hint {
    foreground: u32,
    surface: u32,
}

impl Hint {
    fn new(theme: &Theme) -> Self {
        Self {
            foreground: theme.foreground,
            surface: theme.surface,
        }
    }

    fn with(
        self,
        text: impl Into<SharedString>,
    ) -> impl Fn(&mut Window, &mut App) -> AnyView + 'static {
        let text = text.into();
        move |_, cx| {
            cx.new(|_| crate::usage::Hint {
                text: text.clone(),
                foreground: self.foreground,
                surface: self.surface,
            })
            .into()
        }
    }
}

#[cfg(test)]
mod tests;
