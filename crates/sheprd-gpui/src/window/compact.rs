//! Width-specific touch chrome. Saved sidebar and tab preferences stay intact.
use crate::{
    OwnedNavigationTarget, SheprdWindow,
    browser::{Pick, TabId},
    config::StatusIcons,
    direction::Logical,
    i18n::fl,
    ipad_keys::{band::KEY_ROW, catalog::Key},
    sidebar::{Indicators, status_indicator},
};
use gpui::{prelude::*, *};
use sheprd_client::protocol::AgentStatus;

const COMPACT_FONT: f32 = 13.;
const TOP_BAR: f32 = 52.;

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct OverlaySelection {
    core: Option<(TabId, usize)>,
    endpoint: Option<String>,
    navigation: Option<OwnedNavigationTarget>,
    pane: Option<String>,
    pick: Option<Pick>,
    workspace: Option<String>,
}

impl SheprdWindow {
    pub(crate) fn ipad_compact(&self) -> bool {
        crate::ipad_ui::layout() && self.viewport_width < crate::ipad_ui::metrics::COMPACT_BELOW
    }

    pub(crate) fn ipad_overlay_width(&self, window: &Window) -> Option<f32> {
        if !self.ipad_compact() || self.ipad.overlay_open.shown(crate::motion::now()) <= 0. {
            return None;
        }
        Some(
            crate::ipad_ui::metrics::SIDEBAR
                .min(f32::from(window.fully_visible_bounds().size.width)),
        )
    }

    pub(crate) fn ipad_compact_header(
        &self,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> Option<Div> {
        if !self.ipad_compact() || self.zen_active() {
            return None;
        }
        let group = self.active_group();
        let pick = group.and_then(|group| self.group_pick(group));
        let tabs = group.map_or_else(
            || self.open_workspace_tabs(cx),
            |group| self.group_tabs(group, cx),
        );
        let position = pick
            .as_ref()
            .and_then(|pick| tabs.iter().position(|tab| tab == pick))
            .unwrap_or(0);
        let (name, status) = match &pick {
            Some(Pick::Herdr(id)) => self
                .live
                .snapshot
                .as_ref()
                .and_then(|snapshot| snapshot.tabs.iter().find(|tab| tab.tab_id == *id))
                .map(|tab| (tab.label.clone(), tab.agent_status)),
            Some(Pick::Page(_)) => self
                .browser_tab_labels(cx)
                .into_iter()
                .find(|(tab, _)| Some(tab) == pick.as_ref())
                .map(|(_, label)| (label.to_string(), AgentStatus::Unknown)),
            None => None,
        }
        .unwrap_or_else(|| (fl!("ipad-keys-compact-terminal"), AgentStatus::Unknown));
        let indicators = Indicators::new(
            self.settings.shared.as_ref(),
            StatusIcons::Dots,
            false,
            self.theme_light,
            &self.theme,
        );
        let controls = crate::window::viewport::window_controls(window);
        // Controls cover the bar's leading corner; its background may run underneath.
        let bar = div()
            .debug_selector(|| "ipad-compact-top-bar".into())
            .flex_none()
            .h(px(TOP_BAR))
            .w_full()
            .row()
            .items_center()
            .gap(px(4.))
            .pl(px(4.) + controls.left)
            .pr(px(4.) + controls.right)
            .bg(rgb(self.theme.surface))
            .border_b_1()
            .border_color(rgb(self.theme.active))
            .child(
                self.ipad_compact_button(
                    "ipad-compact-menu",
                    "icons/panel-left.svg",
                    fl!("ipad-keys-compact-menu"),
                )
                .on_click(cx.listener(|this, _, _, cx| {
                    let open = !this.ipad.overlay_open.is_open();
                    this.set_ipad_overlay(open, cx);
                })),
            )
            .child(
                div()
                    .id("ipad-compact-tabs")
                    .debug_selector(|| "ipad-compact-tabs".into())
                    .role(Role::Button)
                    .aria_label(fl!("ipad-keys-compact-tabs"))
                    .flex_1()
                    .min_w_0()
                    .h(px(crate::ipad_ui::metrics::TOUCH))
                    .row()
                    .items_center()
                    .gap(px(6.))
                    .px(px(4.))
                    .rounded(px(8.))
                    .cursor_pointer()
                    .child(status_indicator(status, &self.config.ui, indicators))
                    .child(div().flex_1().min_w_0().truncate().child(name))
                    .child(div().flex_none().text_size(px(12.)).child(fl!(
                        "ipad-keys-compact-tab-position",
                        index = position + 1,
                        count = tabs.len().max(1)
                    )))
                    .child(
                        svg()
                            .path("icons/chevron-down.svg")
                            .size(px(14.))
                            .flex_none()
                            .text_color(rgb(self.theme.foreground)),
                    )
                    .on_click(cx.listener(|this, _, window, cx| {
                        this.set_ipad_overlay(false, cx);
                        this.open_palette(crate::palette::Filter::Navigation, window, cx);
                    })),
            )
            .child(
                self.ipad_compact_button(
                    "ipad-compact-group-menu",
                    "icons/more.svg",
                    fl!("ipad-keys-compact-group-menu"),
                )
                .on_click(cx.listener(|this, event: &ClickEvent, window, cx| {
                    this.set_ipad_overlay(false, cx);
                    if let Some(group) = this.active_group() {
                        this.open_group_menu(group, event.position(), window, cx);
                    }
                })),
            );
        let pager = self
            .ipad_pane_position(cx)
            .filter(|(_, count)| *count > 1)
            .and_then(|_| self.ipad_pager(window, cx));
        Some(
            div()
                .flex_none()
                .flex()
                .flex_col()
                .child(bar)
                .children(pager),
        )
    }

    fn ipad_compact_button(
        &self,
        id: &'static str,
        icon: &'static str,
        label: String,
    ) -> Stateful<Div> {
        div()
            .id(id)
            .debug_selector(move || id.into())
            .role(Role::Button)
            .aria_label(label)
            .flex_none()
            .size(px(crate::ipad_ui::metrics::TOUCH))
            .flex()
            .items_center()
            .justify_center()
            .rounded(px(8.))
            .cursor_pointer()
            // GPUI paints an svg only with its own text color; it inherits none.
            .child(
                svg()
                    .path(icon)
                    .size(px(20.))
                    .text_color(rgb(self.theme.foreground)),
            )
            .on_mouse_down(MouseButton::Left, |_, window, cx| {
                window.prevent_default();
                cx.stop_propagation();
            })
    }

    fn ipad_overlay_selection(&self, cx: &App) -> OverlaySelection {
        OverlaySelection {
            core: self
                .ipad_core_position(cx)
                .map(|(tab, index, _)| (tab, index)),
            endpoint: self
                .endpoints
                .get(self.selected_endpoint)
                .map(|endpoint| endpoint.id.clone()),
            navigation: self.pending_navigation.clone(),
            pane: self
                .live
                .snapshot
                .as_ref()
                .and_then(|snapshot| snapshot.focused_pane_id.clone()),
            pick: self.active_group().and_then(|group| self.group_pick(group)),
            workspace: self
                .live
                .snapshot
                .as_ref()
                .and_then(|snapshot| snapshot.focused_workspace_id.clone()),
        }
    }

    fn set_ipad_overlay(&mut self, open: bool, cx: &mut Context<Self>) {
        if !crate::ipad_ui::active() {
            return;
        }
        self.ipad
            .overlay_open
            .set(open, crate::motion::now(), cfg!(test));
        if open {
            self.ipad.overlay_selection = Some(self.ipad_overlay_selection(cx));
        }
        cx.notify();
    }

    pub(super) fn ipad_sidebar_overlay(
        &mut self,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        if !self.ipad_compact() {
            self.ipad
                .overlay_open
                .set(false, crate::motion::now(), true);
            self.ipad.overlay_selection = None;
            return None;
        }
        if self.ipad.overlay_open.is_open()
            && (self.menu.page.is_some()
                || self
                    .ipad
                    .overlay_selection
                    .as_ref()
                    .is_some_and(|selection| *selection != self.ipad_overlay_selection(cx)))
        {
            self.set_ipad_overlay(false, cx);
        }
        let now = crate::motion::now();
        let shown = self.ipad.overlay_open.shown(now);
        if self.ipad.overlay_open.moving(now) {
            window.request_animation_frame();
        }
        if shown <= 0. {
            self.ipad.overlay_selection = None;
            return None;
        }
        let visible = window.fully_visible_bounds();
        let width = self.ipad_overlay_width(window)?;
        Some(
            div()
                .id("ipad-compact-overlay")
                .debug_selector(|| "ipad-compact-overlay".into())
                .occlude()
                .absolute()
                .top(visible.top() + px(TOP_BAR))
                .left(visible.left())
                .right(window.viewport_size().width - visible.right())
                .bottom(window.viewport_size().height - visible.bottom())
                .child(
                    div()
                        .id("ipad-compact-scrim")
                        .debug_selector(|| "ipad-compact-scrim".into())
                        .role(Role::Button)
                        .aria_label(fl!("ipad-keys-compact-close-sidebar"))
                        .absolute()
                        .inset_0()
                        .bg(rgb(self.theme.background))
                        .opacity(0.66 * shown)
                        .on_mouse_down(MouseButton::Left, |_, window, cx| {
                            window.prevent_default();
                            cx.stop_propagation();
                        })
                        .on_click(cx.listener(|this, _, _, cx| this.set_ipad_overlay(false, cx))),
                )
                .child(
                    div()
                        .id("ipad-compact-sidebar")
                        .debug_selector(|| "ipad-compact-sidebar".into())
                        .absolute()
                        .top_0()
                        .bottom_0()
                        .map(|panel| {
                            let offset = px(-(1. - shown) * width);
                            if crate::direction::is_rtl() {
                                panel.right(offset)
                            } else {
                                panel.left(offset)
                            }
                        })
                        .w(px(width))
                        .occlude()
                        .overflow_hidden()
                        .bg(rgb(self.theme.sidebar_background()))
                        .child(crate::sidebar::cached_view(&self.sidebar_view, width))
                        .on_click(cx.listener(|this, _, window, cx| {
                            if this.focus.is_focused(window)
                                || this.core_terminals.focused(window, cx)
                                || this.menu.page.is_some()
                            {
                                this.set_ipad_overlay(false, cx);
                            }
                        })),
                )
                .into_any_element(),
        )
    }

    pub(super) fn sync_ipad_font(&mut self, window: &Window, cx: &mut Context<Self>) {
        if self.ipad.font_update_pending || self.ipad_compact() == self.ipad.compact_font.is_some()
        {
            return;
        }
        self.ipad.font_update_pending = true;
        let owner = cx.weak_entity();
        // The hook is after body construction: apply the session font before
        // the next frame computes painting, hit testing, and IME geometry.
        window.on_next_frame(move |window, cx| {
            let _ = owner.update(cx, |this, cx| {
                this.ipad.font_update_pending = false;
                let compact = crate::ipad_ui::layout()
                    && f32::from(window.fully_visible_bounds().size.width)
                        < crate::ipad_ui::metrics::COMPACT_BELOW;
                if compact {
                    if this.ipad.compact_font.is_none() {
                        this.ipad.compact_font = Some(this.config.terminal.size);
                        this.set_terminal_font_size(COMPACT_FONT, cx);
                    }
                } else if let Some(restore) = this.ipad.compact_font.take()
                    && this.config.terminal.size == COMPACT_FONT
                {
                    this.set_terminal_font_size(restore, cx);
                }
            });
        });
        window.request_animation_frame();
    }

    pub(super) fn ipad_compact_layers(
        &self,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let now = crate::motion::now();
        let expanded = self.ipad.more_keys.shown(now);
        if self.ipad.more_keys.moving(now) {
            window.request_animation_frame();
        }
        let mut keys = div()
            .id("ipad-compact-key-row")
            .debug_selector(|| "ipad-compact-key-row".into())
            .h(px(KEY_ROW))
            .flex_none()
            .w_full()
            .flex()
            .items_center()
            .gap(px(4.))
            .px(px(4.))
            .bg(rgb(self.theme.surface))
            .overflow_x_scroll();
        let mut compact: Vec<_> = Key::ALL
            .iter()
            .copied()
            .filter(|key| key.compact().is_some())
            .collect();
        compact.sort_by_key(|key| key.compact());
        for key in compact {
            keys = keys.child(self.ipad_key_button(key, cx));
        }
        if expanded > 0. {
            let rest: Vec<_> = Key::ALL
                .iter()
                .copied()
                .filter(|key| key.compact().is_none())
                .collect();
            let width: f32 = rest.iter().map(|key| key.width() + 4.).sum();
            keys = keys.child(
                div()
                    .flex_none()
                    .flex()
                    .gap(px(4.))
                    .overflow_hidden()
                    .w(px(width * expanded))
                    .opacity(expanded)
                    .children(rest.into_iter().map(|key| self.ipad_key_button(key, cx))),
            );
        }
        keys.into_any_element()
    }
}
