//! Touch-sized rail rows; the desktop mark grid stays in the parent renderer.
use super::*;
use crate::{
    devices_overview::Tally,
    ipad_ui::{accent, metrics::*},
    sidebar::agents::status_dot,
};

impl HerdrWindow {
    /// Chrome outside the sidebar uses the same themed dot without exposing
    /// the sidebar's drawing module.
    pub(crate) fn ipad_rail_dot(&self, status: AgentStatus, diameter: f32) -> Div {
        status_dot(
            status,
            diameter,
            Indicators::new(
                self.settings.shared.as_ref(),
                self.config.status_icons,
                false,
                self.theme_light,
                &self.theme,
            ),
        )
    }

    pub(super) fn render_ipad_rail(
        &self,
        width: f32,
        indicators: Indicators,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        let theme = &self.theme;
        let multi = self.endpoints.len() > 1;
        let mut spaces = section("rail-spaces");
        for (index, endpoint) in self.displayed_devices() {
            if !self.device_visible(&endpoint.id) {
                continue;
            }
            let selected = index == self.selected_endpoint;
            let live = if selected { &self.live } else { &endpoint.live };
            let endpoint_id = endpoint.id.clone();
            let select = endpoint_id.clone();
            let menu = endpoint_id.clone();
            let connected = self.displayed_connection_status(endpoint).is_connected();
            spaces = spaces.child(
                button(format!("rail-host-{endpoint_id}"), selected, theme)
                    .aria_label(endpoint.label.clone())
                    .child(
                        status_dot(
                            if connected {
                                AgentStatus::Done
                            } else {
                                AgentStatus::Unknown
                            },
                            RAIL_DOT,
                            indicators,
                        )
                        .debug_selector(|| format!("rail-host-status-{endpoint_id}")),
                    )
                    .child(label(
                        format!("rail-host-label-{endpoint_id}"),
                        &endpoint.label,
                    ))
                    .map(|row| {
                        context_menu(
                            row,
                            cx.listener(move |this, event: &ContextMenuEvent, window, cx| {
                                cx.stop_propagation();
                                this.open_rail_row_menu(&menu, None, event.position, window, cx);
                                this.menu.opening_right_click = event.mouse.is_some()
                                    && matches!(
                                        this.menu.page,
                                        Some(crate::menu::Page::RowMenu(_))
                                    );
                            }),
                        )
                    })
                    .on_click(cx.listener(move |this, _, window, cx| {
                        this.select_endpoint(&select, cx);
                        window.focus(&this.focus, cx);
                    })),
            );
            if multi && endpoint.collapsed {
                continue;
            }
            let Some(snapshot) = live.snapshot.as_deref() else {
                continue;
            };
            // Count once per host rather than scanning all agents for each row.
            let mut tallies = HashMap::<&str, Tally>::new();
            for agent in &snapshot.agents {
                let tally = tallies.entry(&agent.workspace_id).or_default();
                *tally = tally.add(Tally::of([agent.agent_status]));
            }
            let collapsed = if index == 0 {
                &self.collapsed_repos
            } else {
                &endpoint.collapsed_repos
            };
            for (row, indented, _) in visible_workspace_entries(&snapshot.workspaces, collapsed) {
                let workspace = &snapshot.workspaces[row];
                let id = workspace.workspace_id.clone();
                let key = format!("rail-workspace-{endpoint_id}-{id}");
                let navigate = endpoint_id.clone();
                let menu_endpoint = endpoint_id.clone();
                let menu_workspace = id.clone();
                let name = workspace_label(workspace, indented);
                let blocked = tallies.get(id.as_str()).map_or(0, |tally| tally.blocked);
                spaces = spaces.child(
                    button(key.clone(), selected && workspace.focused, theme)
                        .aria_label(name.to_owned())
                        .child(
                            self.ipad_rail_dot(workspace.agent_status, RAIL_DOT)
                                .debug_selector(|| format!("rail-dot-{key}")),
                        )
                        .child(
                            label(format!("rail-label-{key}"), name)
                                .text_font(&self.config.terminal),
                        )
                        .when(blocked > 0, |row| {
                            row.child(badge(format!("rail-badge-{key}"), blocked, theme))
                        })
                        .map(|row| {
                            context_menu(
                                row,
                                cx.listener(move |this, event: &ContextMenuEvent, window, cx| {
                                    cx.stop_propagation();
                                    this.open_rail_row_menu(
                                        &menu_endpoint,
                                        Some(NavigationTarget::Workspace(&menu_workspace)),
                                        event.position,
                                        window,
                                        cx,
                                    );
                                    this.menu.opening_right_click = event.mouse.is_some()
                                        && matches!(
                                            this.menu.page,
                                            Some(crate::menu::Page::RowMenu(_))
                                        );
                                }),
                            )
                        })
                        .on_click(cx.listener(move |this, _, window, cx| {
                            this.navigate_endpoint(&navigate, NavigationTarget::Workspace(&id), cx);
                            window.focus(&this.focus, cx);
                        })),
                );
            }
        }
        let mut agents = section("rail-agents");
        if self.config.show_agents {
            for (index, agent) in self.panel_agents() {
                let Some(endpoint) = self.endpoints.get(index) else {
                    continue;
                };
                if multi && endpoint.collapsed {
                    continue;
                }
                let endpoint_id = endpoint.id.clone();
                let pane = agent.pane_id.clone();
                let key = format!("rail-agent-{endpoint_id}-{pane}");
                let menu_endpoint = endpoint_id.clone();
                let menu_pane = pane.clone();
                agents = agents.child(
                    button(
                        key.clone(),
                        index == self.selected_endpoint && agent.focused,
                        theme,
                    )
                    .aria_label(agent_name(agent).to_owned())
                    .child(
                        div()
                            .relative()
                            .size(px(RAIL_ICON))
                            .flex_none()
                            .debug_selector(|| format!("rail-agent-icon-{endpoint_id}-{pane}"))
                            .child(
                                crate::icons::of_agent(agent).element(RAIL_ICON, theme.subtext()),
                            )
                            .child(
                                div()
                                    .absolute()
                                    .inset_e(px(-RAIL_RING))
                                    .bottom(px(-RAIL_RING))
                                    .debug_selector(|| {
                                        format!("rail-agent-ring-{endpoint_id}-{pane}")
                                    })
                                    .size(px(DOT + 2. * RAIL_RING))
                                    .p(px(RAIL_RING))
                                    .rounded_full()
                                    .bg(rgb(theme.sidebar_background()))
                                    .child(
                                        status_dot(agent.agent_status, DOT, indicators)
                                            .debug_selector(|| format!("rail-dot-{key}")),
                                    ),
                            ),
                    )
                    .child(label(format!("rail-label-{key}"), agent_name(agent)))
                    .map(|row| {
                        context_menu(
                            row,
                            cx.listener(move |this, event: &ContextMenuEvent, window, cx| {
                                cx.stop_propagation();
                                this.open_rail_row_menu(
                                    &menu_endpoint,
                                    Some(NavigationTarget::Pane(&menu_pane)),
                                    event.position,
                                    window,
                                    cx,
                                );
                                this.menu.opening_right_click = event.mouse.is_some()
                                    && matches!(
                                        this.menu.page,
                                        Some(crate::menu::Page::RowMenu(_))
                                    );
                            }),
                        )
                    })
                    .on_click(cx.listener(move |this, _, window, cx| {
                        this.navigate_endpoint(&endpoint_id, NavigationTarget::Pane(&pane), cx);
                        window.focus(&this.focus, cx);
                    })),
                );
            }
        }
        let split = self.sidebar_split.unwrap_or(0.5).clamp(0.1, 0.9);
        spaces.style().flex_grow = Some(if self.config.show_agents { split } else { 1. });
        agents.style().flex_grow = Some(1. - split);
        div()
            .id("sidebar-rail")
            .debug_selector(|| "sidebar-rail".into())
            .w(px(width))
            .h_full()
            .min_h_0()
            .flex_none()
            .flex()
            .flex_col()
            .items_center()
            .overflow_hidden()
            .text_font(&self.config.sidebar)
            .text_size(px(RAIL_LABEL))
            .line_height(px(RAIL_LABEL + 4.))
            .text_color(rgb(theme.foreground))
            .bg(self.fill().panel(theme.sidebar_background()))
            .border_e(px(1.))
            .border_color(rgb(theme.active))
            .child(spaces)
            .when(self.config.show_agents, |rail| {
                rail.child(
                    div()
                        .debug_selector(|| "ipad-rail-divider".into())
                        .flex_none()
                        .w(px(RAIL_DIVIDER))
                        .h(px(1.))
                        .bg(rgb(theme.active)),
                )
                .child(agents)
            })
            .child(
                button("rail-new-workspace".into(), false, theme)
                    .aria_label(fl!("sidebar-new-workspace"))
                    .child(label_text("+"))
                    .on_click(cx.listener(|this, _, window, cx| {
                        this.command(Command::Workspace, window, cx)
                    })),
            )
            .child(
                button("rail-expand".into(), false, theme)
                    .aria_label(fl!("ipad-rail-expand"))
                    .child(label_text("»"))
                    .on_click(cx.listener(|this, _, window, cx| {
                        this.command(Command::ToggleSidebar, window, cx)
                    })),
            )
    }
}

fn section(id: &'static str) -> Stateful<Div> {
    div()
        .id(id)
        .debug_selector(move || id.into())
        .flex()
        .flex_col()
        .items_center()
        .flex_1()
        .min_h_0()
        .w_full()
        .overflow_y_scroll()
}

fn button(id: String, selected: bool, theme: &Theme) -> Stateful<Div> {
    div()
        .id(SharedString::from(id.clone()))
        .debug_selector(move || id)
        .role(Role::Button)
        .aria_selected(selected)
        .relative()
        .flex_none()
        .w(px(RAIL_BUTTON_W))
        .h(px(RAIL_BUTTON_H))
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .gap(px(5.))
        .rounded(px(6.))
        .border_1()
        .border_color(if selected {
            Hsla::from(rgb(accent(theme)))
        } else {
            transparent_black()
        })
        .when(selected, |row| row.bg(rgb(theme.active)))
        .cursor_pointer()
}

fn label(id: String, text: &str) -> Div {
    div()
        .debug_selector(move || id)
        .w_full()
        .min_w_0()
        .px(px(3.))
        .text_size(px(RAIL_LABEL))
        .text_center()
        .truncate()
        .child(text.to_owned())
}

fn badge(id: String, count: usize, theme: &Theme) -> Stateful<Div> {
    div()
        .id(SharedString::from(id.clone()))
        .role(Role::Label)
        .debug_selector(move || id)
        .absolute()
        .top(px(2.))
        .inset_e(px(2.))
        .size(px(RAIL_BADGE))
        .rounded_full()
        .flex()
        .items_center()
        .justify_center()
        .bg(rgb(theme.palette[3]))
        .text_color(rgb(theme.text_on(theme.palette[3])))
        .text_size(px(RAIL_LABEL))
        .line_height(px(RAIL_BADGE))
        .aria_label(fl!("ipad-rail-needs-you-count", count = count))
        .child(count.to_string())
}
