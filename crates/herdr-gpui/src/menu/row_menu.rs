//! The compact rail's touch menu, retaining stable identities until activation.
use super::Page;
use crate::{
    Command, HerdrWindow, NavigationTarget,
    devices_overview::Tally,
    fonts::StyledFont,
    i18n::fl,
    ipad_ui::metrics::{MENU_RADIUS, MENU_ROW, MENU_WIDTH, RAIL_DOT},
    navigation::OwnedNavigationTarget,
    sidebar::{agent_label, workspace_label},
};
use gpui::{prelude::*, *};
use herdr_client::protocol::AgentStatus;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RowTarget {
    // keep-sorted start
    Agent,
    Host,
    Space,
    // keep-sorted end
}

#[derive(Clone)]
pub(super) struct State {
    endpoint: String,
    generation: u64,
    epoch: u64,
    focus: FocusHandle,
    tab: Option<String>,
    boot: Option<String>,
    target: Option<OwnedNavigationTarget>,
    workspace: Option<String>,
    label: String,
    status: AgentStatus,
    blocked: usize,
}

impl State {
    fn kind(&self) -> RowTarget {
        match self.target {
            None => RowTarget::Host,
            Some(NavigationTarget::Workspace(_)) => RowTarget::Space,
            Some(_) => RowTarget::Agent,
        }
    }

    fn current(&self, view: &HerdrWindow) -> bool {
        let endpoint = &view.endpoints[view.selected_endpoint];
        if endpoint.id != self.endpoint
            || endpoint.generation != self.generation
            || view.selection_epoch != self.epoch
        {
            return false;
        }
        let snapshot = view.live.snapshot.as_deref();
        if snapshot.map(|snapshot| &snapshot.boot_id) != self.boot.as_ref() {
            return false;
        }
        match self.target.as_ref().map(NavigationTarget::as_deref) {
            None => true,
            Some(NavigationTarget::Workspace(id)) => snapshot.is_some_and(|snapshot| {
                snapshot
                    .workspaces
                    .iter()
                    .any(|workspace| workspace.workspace_id == id)
            }),
            Some(NavigationTarget::Pane(id)) => snapshot.is_some_and(|snapshot| {
                snapshot.agents.iter().any(|agent| {
                    agent.pane_id == id
                        && Some(&agent.workspace_id) == self.workspace.as_ref()
                        && Some(&agent.tab_id) == self.tab.as_ref()
                })
            }),
            Some(NavigationTarget::Tab(_)) => false,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Action {
    // keep-sorted start
    Detach,
    NewTab,
    Open,
    OpenChat,
    // keep-sorted end
}

impl Action {
    fn rows(target: RowTarget) -> &'static [Self] {
        match target {
            RowTarget::Host => &[Self::Open, Self::NewTab, Self::OpenChat, Self::Detach],
            RowTarget::Space | RowTarget::Agent => &[Self::Open, Self::NewTab, Self::OpenChat],
        }
    }

    fn label(self, host: &str) -> String {
        match self {
            Self::Open => fl!("ipad-rail-row-menu-open"),
            Self::NewTab => fl!("ipad-rail-row-menu-new-tab"),
            Self::OpenChat => fl!("ipad-rail-row-menu-open-chat"),
            Self::Detach => fl!("ipad-rail-row-menu-detach", host = host),
        }
    }

    fn command(self) -> Option<Command> {
        match self {
            Self::NewTab => Some(Command::Tab),
            Self::Detach => Some(Command::Detach),
            _ => None,
        }
    }
}

impl HerdrWindow {
    pub(crate) fn open_rail_row_menu(
        &mut self,
        endpoint: &str,
        target: Option<NavigationTarget<&str>>,
        anchor: Point<Pixels>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !crate::ipad_ui::layout()
            || self.menu.page.is_some()
            || !self.select_endpoint(endpoint, cx)
        {
            return;
        }
        let snapshot = self.live.snapshot.as_deref();
        let (workspace, label, status) = match target.as_ref() {
            None => (
                snapshot
                    .and_then(|snapshot| {
                        snapshot
                            .workspaces
                            .iter()
                            .find(|workspace| workspace.focused)
                    })
                    .map(|workspace| workspace.workspace_id.clone()),
                self.endpoints[self.selected_endpoint].label.clone(),
                if self.live.status.is_connected() {
                    AgentStatus::Done
                } else {
                    AgentStatus::Unknown
                },
            ),
            Some(NavigationTarget::Workspace(id)) => {
                let Some(workspace) = snapshot.and_then(|snapshot| {
                    snapshot
                        .workspaces
                        .iter()
                        .find(|workspace| workspace.workspace_id == *id)
                }) else {
                    return;
                };
                (
                    Some(workspace.workspace_id.clone()),
                    workspace_label(workspace, false).to_owned(),
                    workspace.agent_status,
                )
            }
            Some(NavigationTarget::Pane(id)) => {
                let Some(agent) = snapshot
                    .and_then(|snapshot| snapshot.agents.iter().find(|agent| agent.pane_id == *id))
                else {
                    return;
                };
                (
                    Some(agent.workspace_id.clone()),
                    snapshot
                        .map(|snapshot| agent_label(agent, snapshot))
                        .unwrap_or_default(),
                    agent.agent_status,
                )
            }
            Some(NavigationTarget::Tab(_)) => return,
        };
        let blocked = Tally::of(
            snapshot
                .into_iter()
                .flat_map(|snapshot| &snapshot.agents)
                .filter(|agent| match target.as_ref() {
                    None => true,
                    Some(NavigationTarget::Workspace(id)) => agent.workspace_id == *id,
                    Some(NavigationTarget::Pane(id)) => agent.pane_id == *id,
                    Some(NavigationTarget::Tab(_)) => false,
                })
                .map(|agent| agent.agent_status),
        )
        .blocked;
        let state = State {
            endpoint: endpoint.to_owned(),
            generation: self.endpoints[self.selected_endpoint].generation,
            epoch: self.selection_epoch,
            focus: cx.focus_handle(),
            tab: match target.as_ref() {
                Some(NavigationTarget::Pane(id)) => snapshot
                    .and_then(|snapshot| snapshot.agents.iter().find(|agent| agent.pane_id == *id))
                    .map(|agent| agent.tab_id.clone()),
                _ => None,
            },
            boot: snapshot.map(|snapshot| snapshot.boot_id.clone()),
            target: target.as_ref().map(Into::into),
            workspace,
            label,
            status,
            blocked,
        };
        if !self.open_menu(window, cx) {
            return;
        }
        self.menu.anchor = anchor;
        window.focus(&state.focus, cx);
        self.menu.page = Some(Page::RowMenu(state.kind()));
        self.menu.row_menu = Some(state);
        cx.notify();
    }

    pub(super) fn render_row_menu(
        &self,
        panel: Stateful<Div>,
        target: RowTarget,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        let Some(state) = self
            .menu
            .row_menu
            .as_ref()
            .filter(|state| state.kind() == target)
        else {
            return panel;
        };
        let theme = &self.theme;
        let room = window.fully_visible_bounds().size;
        panel
            .w(px(MENU_WIDTH).min((room.width - px(24.)).max(px(0.))))
            .max_h((room.height - px(24.)).max(px(0.)))
            .p_0()
            .rounded(px(MENU_RADIUS))
            .flex()
            .flex_col()
            .shadow_lg()
            .track_focus(&state.focus)
            .on_key_down(cx.listener(Self::rail_row_key))
            .child(
                div()
                    .debug_selector(|| "ipad-rail-row-menu-header".into())
                    .flex_none()
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .p(px(12.))
                    .border_b_1()
                    .border_color(rgb(theme.active))
                    .child(self.ipad_rail_dot(state.status, RAIL_DOT).flex_none())
                    .child(
                        div()
                            .min_w_0()
                            .flex_1()
                            .flex()
                            .flex_col()
                            .child(
                                div()
                                    .debug_selector(|| "ipad-rail-row-menu-name".into())
                                    .truncate()
                                    .child(state.label.clone()),
                            )
                            .child(
                                div()
                                    .debug_selector(|| "ipad-rail-row-menu-count".into())
                                    .text_color(rgb(theme.subtext()))
                                    .text_size(px(10.))
                                    .child(fl!("ipad-rail-needs-you-count", count = state.blocked)),
                            ),
                    ),
            )
            .children(
                Action::rows(target)
                    .iter()
                    .copied()
                    .enumerate()
                    .map(|(index, action)| {
                        let id = format!("ipad-rail-row-menu-{action:?}");
                        let label = action.label(&state.label);
                        div()
                            .id(SharedString::from(id.clone()))
                            .debug_selector(move || id)
                            .role(Role::MenuItem)
                            .aria_label(label.clone())
                            .when(self.menu.selected == Some(index), |row| {
                                row.aria_active_descendant().bg(rgb(theme.active))
                            })
                            .flex_none()
                            .h(px(MENU_ROW))
                            .w_full()
                            .px(px(12.))
                            .flex()
                            .items_center()
                            .cursor_pointer()
                            .text_font(&self.config.ui)
                            .text_color(rgb(theme.foreground))
                            .child(div().min_w_0().truncate().child(label))
                            .on_click(cx.listener(move |this, _, window, cx| {
                                this.activate_rail_row(action, window, cx)
                            }))
                    }),
            )
    }

    fn activate_rail_row(&mut self, action: Action, window: &mut Window, cx: &mut Context<Self>) {
        let Some(state) = self.menu.row_menu.clone() else {
            return;
        };
        if !state.current(self) || !Action::rows(state.kind()).contains(&action) {
            self.dismiss_menu(window, cx);
            return;
        }
        let chat = if action == Action::OpenChat {
            self.live.snapshot.as_deref().and_then(|snapshot| {
                snapshot
                    .agents
                    .iter()
                    .filter(|agent| Some(&agent.workspace_id) == state.workspace.as_ref())
                    .filter(|agent| match state.target.as_ref() {
                        Some(NavigationTarget::Pane(pane)) => agent.pane_id == *pane,
                        _ => true,
                    })
                    .find(|agent| {
                        self.chats.pane(&agent.pane_id).is_some_and(|chat| {
                            chat.available()
                                && agent
                                    .tokens
                                    .iter()
                                    .find(|(key, _)| key == herdr_chat_protocol::TOKEN)
                                    .and_then(|(_, value)| {
                                        value.parse::<herdr_chat_protocol::EndpointId>().ok()
                                    })
                                    .is_some_and(|endpoint| endpoint == chat.target.endpoint)
                        })
                    })
                    .map(|agent| agent.pane_id.clone())
            })
        } else {
            None
        };
        let anchor = self.menu.anchor;
        self.dismiss_menu(window, cx);
        if action == Action::NewTab || (action == Action::OpenChat && chat.is_none()) {
            self.rail_workspace_action(state, action, anchor, window, cx);
            return;
        }
        if let Some(pane) = chat {
            self.chats.set_raw(&pane, false);
            self.navigate_endpoint(&state.endpoint, NavigationTarget::Pane(&pane), cx);
            return;
        }
        if action == Action::Detach {
            if let Some(command) = action.command() {
                self.command(command, window, cx);
            }
            return;
        }
        let destination = state
            .target
            .or_else(|| state.workspace.clone().map(NavigationTarget::Workspace));
        let Some(destination) = destination else {
            return;
        };
        if !self.navigate_endpoint(&state.endpoint, destination.as_deref(), cx) {
            return;
        }
        window.focus(&self.focus, cx);
    }
}

impl HerdrWindow {
    fn rail_row_key(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        let Some(state) = &self.menu.row_menu else {
            return;
        };
        let actions = Action::rows(state.kind());
        let action = match event.keystroke.key.as_str() {
            "escape" => {
                self.dismiss_menu(window, cx);
                None
            }
            "up" | "down" => {
                let index = match (self.menu.selected, event.keystroke.key.as_str()) {
                    (None, "up") => actions.len() - 1,
                    (None, _) => 0,
                    (Some(index), "up") => (index + actions.len() - 1) % actions.len(),
                    (Some(index), _) => (index + 1) % actions.len(),
                };
                self.menu.selected = Some(index);
                cx.notify();
                None
            }
            "enter" => self
                .menu
                .selected
                .and_then(|index| actions.get(index))
                .copied(),
            _ => None,
        };
        cx.stop_propagation();
        window.prevent_default();
        if let Some(action) = action {
            self.activate_rail_row(action, window, cx);
        }
    }

    fn rail_workspace_action(
        &mut self,
        state: State,
        action: Action,
        anchor: Point<Pixels>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(workspace) = state.workspace.clone() else {
            return;
        };
        let focused = self.live.snapshot.as_deref().is_some_and(|snapshot| {
            snapshot.focused_workspace_id.as_deref() == Some(workspace.as_str())
        });
        if focused && self.input_ready() && self.activation_deadline.is_none() {
            // This choice supersedes an older deferred focus, as navigation does.
            self.pending_navigation = None;
            self.finish_rail_workspace_action(action, anchor, window, cx);
            return;
        }
        if !self.navigate_endpoint(&state.endpoint, NavigationTarget::Workspace(&workspace), cx) {
            return;
        }
        let timer = cx.background_executor().clone();
        cx.spawn_in(window, async move |this, cx| {
            // Never create against the pre-navigation snapshot. A replaced host,
            // another menu, or another requested focus cancels this operation.
            for _ in 0..100 {
                let waiting = this.update_in(cx, |this, window, cx| {
                    if !state.current(this) || this.menu.page.is_some() { return false; }
                    if this.pending_navigation.as_ref().is_some_and(|focus|
                        !matches!(focus, NavigationTarget::Workspace(id) if id == &workspace)) {
                        return false;
                    }
                    if this.live.activation.as_ref().and_then(|activation| activation.focus.as_ref())
                        .is_some_and(|focus| !matches!(focus, NavigationTarget::Workspace(id) if id == &workspace)) {
                        return false;
                    }
                    if this.input_ready() && this.activation_deadline.is_none()
                        && this.live.snapshot.as_deref().is_some_and(|snapshot|
                            snapshot.focused_workspace_id.as_deref() == Some(workspace.as_str())) {
                        this.finish_rail_workspace_action(action, anchor, window, cx);
                        return false;
                    }
                    true
                });
                if !matches!(waiting, Ok(true)) { return; }
                timer.timer(std::time::Duration::from_millis(50)).await;
            }
        }).detach();
    }
    fn finish_rail_workspace_action(
        &mut self,
        action: Action,
        anchor: Point<Pixels>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match action {
            Action::NewTab => {
                if let Some(command) = action.command() {
                    self.command(command, window, cx);
                }
            }
            Action::OpenChat => {
                if let Some(group) = self
                    .active_group()
                    .or_else(|| self.group_slots().first().map(|slot| slot.id))
                {
                    self.open_group_menu(group, anchor, window, cx);
                }
            }
            Action::Open | Action::Detach => {}
        }
    }
}
