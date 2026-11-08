use super::*;
use crate::{
    endpoint::origin::Origin,
    i18n::fl,
    ipad_ui::metrics::{DOT, TOUCH},
};

const HOST_ROW: f32 = 64.;

impl SheprdWindow {
    pub(super) fn ipad_host_row(
        &self,
        endpoint: &Endpoint,
        selected: bool,
        look: SidebarLook,
        width: f32,
        pinned: bool,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        let theme = &self.theme;
        let prefix = if pinned { "sticky-" } else { "" };
        let id = endpoint.id.clone();
        let collapse = id.clone();
        let select = id.clone();
        let menu = id.clone();
        let connected = self.displayed_connection_status(endpoint).is_connected();
        let origin = match Origin::of(&endpoint.connection.target) {
            Origin::Local => "local",
            Origin::Ssh => "ssh",
            Origin::Tailscale => "tailscale",
            Origin::Wsl => "wsl",
            #[cfg(feature = "cloud")]
            Origin::Cloud { .. } => "cloud",
        };
        let generation = match endpoint.backend_generation() {
            sheprd_backend::Generation::Herdr1 => 1,
            sheprd_backend::Generation::Herdr2 => 2,
        };
        let detail = format!(
            "{} · {}",
            fl!("ipad-sidebar-host-origin", origin = origin),
            fl!("ipad-sidebar-host-generation", generation = generation)
        );
        let label_width = (look.content_width(width) - DOT - 16. - TOUCH).max(0.);
        let row = div()
            .id(SharedString::from(format!("{prefix}host-{id}")))
            .debug_selector(|| format!("{prefix}host-{id}"))
            .h(px(HOST_ROW))
            .flex_none()
            .relative()
            .row()
            .items_center()
            .px(px(look.content_x()))
            .gap(px(8.))
            .text_color(rgb(theme.foreground))
            .when(pinned, |row| {
                row.bg(rgb(theme.sidebar_background()))
                    .block_mouse_except_scroll()
            })
            .map(|row| {
                let mark = wash::HostMark::resolve(
                    &self.config.sidebar_style,
                    &endpoint.label,
                    selected,
                    theme,
                );
                let layer = look.highlight(
                    &format!("{prefix}host-{id}"),
                    RowState {
                        selected,
                        ..Default::default()
                    },
                    theme,
                );
                look.washed(row, &format!("{prefix}host-{id}"), mark, layer)
            })
            .when(self.device_removing(endpoint), |row| row.opacity(0.5))
            .child(
                div()
                    .debug_selector(|| format!("{prefix}host-status-{id}"))
                    .size(px(DOT))
                    .flex_none()
                    .rounded_full()
                    .border_1()
                    .border_color(rgb(if connected {
                        theme.ink(theme.palette[2])
                    } else {
                        theme.subtext()
                    }))
                    .when(connected, |dot| dot.bg(rgb(theme.ink(theme.palette[2])))),
            )
            .child(
                div()
                    .w(px(label_width))
                    .flex_none()
                    .overflow_hidden()
                    .flex()
                    .flex_col()
                    .child(
                        div()
                            .debug_selector(|| format!("{prefix}host-label-{id}"))
                            .font_weight(FontWeight::BOLD)
                            .truncate()
                            .child(endpoint.label.clone()),
                    )
                    .child(
                        div()
                            .debug_selector(|| format!("{prefix}host-detail-{id}"))
                            .text_size(px((self.config.sidebar.size * 0.9).round()))
                            .text_color(rgb(theme.subtext()))
                            .truncate()
                            .child(detail),
                    ),
            )
            .child(
                div()
                    .id(SharedString::from(format!("{prefix}collapse-host-{id}")))
                    .debug_selector(|| format!("{prefix}collapse-host-{id}"))
                    .size(px(TOUCH))
                    .flex_none()
                    .flex()
                    .items_center()
                    .justify_center()
                    .cursor_pointer()
                    .child(label_text(if endpoint.collapsed {
                        crate::direction::glyph("▸")
                    } else {
                        "▾"
                    }))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        cx.stop_propagation();
                        this.toggle_device(&collapse, cx);
                    })),
            );
        let row = if pinned {
            row
        } else {
            let row = row
                .role(Role::ListItem)
                .aria_label(crate::sidebar::accessibility::device_name(
                    &endpoint.label,
                    connected,
                    None,
                    crate::notification_mutes::Mutes::of(cx)
                        .is_some_and(|mutes| mutes.host(&endpoint.id)),
                ))
                .aria_selected(selected)
                .map(|control| {
                    self.chrome_stop(crate::window::chrome_focus::Region::Sidebar, control)
                });
            let endpoint_id = id.clone();
            crate::sidebar::accessibility::expandable(
                row,
                !endpoint.collapsed,
                cx.entity().downgrade(),
                move |this, _, cx| this.toggle_device(&endpoint_id, cx),
            )
        };
        context_menu(
            row,
            cx.listener(move |this, event: &ContextMenuEvent, window, cx| {
                cx.stop_propagation();
                this.open_host_menu(&menu, event.position, window, cx);
                this.menu.opening_right_click =
                    event.mouse.is_some() && this.menu.page == Some(crate::menu::Page::Host);
            }),
        )
        .on_click(cx.listener(move |this, _, window, cx| {
            this.select_endpoint(&select, cx);
            window.focus(&this.focus, cx);
        }))
    }
}
