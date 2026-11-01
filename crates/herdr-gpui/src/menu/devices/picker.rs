//! Device picker rows and progress, using the same visible-device list as
//! selection and keyboard navigation.

use super::{
    DeviceScope, SetupUnavailable, add_device_offer, colors, device_list_height, multi_select_hint,
};
use crate::{HerdrWindow, i18n::fl};
use gpui::{prelude::*, *};
use herdr_client::ConnectTarget;

impl HerdrWindow {
    pub(in crate::menu) fn render_devices(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let connected = self.connected_picker_devices();
        let count = self.picker_endpoints().count();
        let mut rows = vec![(
            fl!("devices-all"),
            fl!("devices-all-detail", count = count, connected = connected),
            self.device_filter == DeviceScope::All,
            true,
        )];
        rows.extend(self.picker_endpoints().map(|endpoint| {
            let detail = match &endpoint.connection.target {
                // Names: the host, machine, or distribution, and the session.
                // i18n: data
                ConnectTarget::Ssh { target, session } => format!("{target} · {session}"),
                #[cfg(feature = "cloud")]
                ConnectTarget::Cloud {
                    provider,
                    machine,
                    session,
                    ..
                } => format!("{} {machine} · {session}", crate::cloud::name(*provider)),
                // i18n: data
                ConnectTarget::Wsl { distro, session } => format!("WSL {distro} · {session}"),
                ConnectTarget::Socket(path) => path.display().to_string(),
                ConnectTarget::Session { name, .. } => {
                    fl!("devices-this-device-session", session = name.as_str())
                }
                ConnectTarget::Local => fl!("devices-this-device"),
            };
            (
                endpoint.label.clone(),
                fl!(
                    "devices-row-detail",
                    place = detail,
                    state = self.picker_connection_text(endpoint)
                ),
                self.device_filter.picks(&endpoint.id),
                endpoint.enabled,
            )
        }));
        rows.push((
            fl!("devices-add"),
            self.device_setup_unavailable().map_or_else(
                || add_device_offer(self.cloud_offered()),
                SetupUnavailable::text,
            ),
            false,
            self.device_setup_unavailable().is_none(),
        ));
        if self.sessions_available() {
            rows.push((
                fl!("devices-sessions-row"),
                if cfg!(target_os = "ios") {
                    fl!("devices-remote-sessions-detail")
                } else {
                    fl!("devices-sessions-detail")
                },
                false,
                true,
            ));
        }
        let mut view = div()
            .id("devices-list")
            .max_h(device_list_height(self.menu.anchor.y))
            .overflow_y_scroll()
            .track_scroll(&self.menu.devices_scroll)
            .flex()
            .flex_col()
            .gap(px(4.))
            .child(
                div()
                    .p(px(8.))
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap(px(8.))
                    .text_color(rgb(self.theme.muted))
                    .child(div().flex_none().child(fl!("devices-heading")))
                    .child(
                        // Wraps rather than cutting a longer language short.
                        div()
                            .min_w_0()
                            .text_size(px(self.config.ui.size * 0.85))
                            .child(multi_select_hint()),
                    ),
            );
        for (index, (label, detail, checked, enabled)) in rows.into_iter().enumerate() {
            let endpoint = index
                .checked_sub(1)
                .and_then(|i| self.picker_endpoints().nth(i));
            view = view.child(
                div()
                    .id(("device-row", index))
                    .debug_selector(move || format!("device-row-{index}"))
                    .p(px(8.))
                    .rounded(px(crate::config::corners::CONTROL))
                    .flex_none()
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .when(self.menu.selected == Some(index), |s| {
                        s.bg(rgb(self.theme.active))
                    })
                    // The current scope is highlighted as the session list marks
                    // its current session, rather than with a trailing check.
                    .when(checked, |s| s.bg(rgb(self.theme.primary_wash())))
                    .when(enabled, |s| s.cursor_pointer())
                    .text_color(rgb(if enabled {
                        self.theme.foreground
                    } else {
                        self.theme.muted
                    }))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .justify_between()
                                    .gap(px(8.))
                                    .when(index == 0, |row| {
                                        row.child(
                                            svg()
                                                .path("icons/devices.svg")
                                                .size(px(16.))
                                                .flex_none()
                                                .text_color(rgb(self.theme.foreground)),
                                        )
                                    })
                                    .when(index == self.session_list_row(), |row| {
                                        row.child(
                                            svg()
                                                .path("icons/sessions.svg")
                                                .size(px(16.))
                                                .flex_none()
                                                .text_color(rgb(self.theme.foreground)),
                                        )
                                    })
                                    .when(index == self.add_device_row(), |row| {
                                        row.child(
                                            svg()
                                                .debug_selector(|| "device-add".into())
                                                .path("icons/plus.svg")
                                                .size(px(16.))
                                                .flex_none()
                                                .text_color(rgb(if enabled {
                                                    self.theme.foreground
                                                } else {
                                                    self.theme.muted
                                                })),
                                        )
                                    })
                                    .child(
                                        div()
                                            .flex_1()
                                            .min_w_0()
                                            .truncate()
                                            .when(checked, |label| {
                                                label
                                                    .debug_selector(move || {
                                                        format!("device-current-{index}")
                                                    })
                                                    .font_weight(FontWeight::SEMIBOLD)
                                            })
                                            .child(label),
                                    ),
                            )
                            .child(
                                div()
                                    .text_size(px(self.config.ui.size * 0.85))
                                    .text_color(rgb(self.theme.muted))
                                    .child(detail),
                            ),
                    )
                    .when_some(endpoint, |row, endpoint| {
                        row.child(
                            div()
                                .debug_selector(move || format!("device-dot-{index}"))
                                .size(px(7.))
                                .flex_none()
                                .rounded_full()
                                .bg(rgb(
                                    if self.displayed_connection_status(endpoint).is_connected() {
                                        colors::online(&self.theme)
                                    } else {
                                        self.theme.muted
                                    },
                                )),
                        )
                    })
                    .on_hover(cx.listener(move |this, hovered, _, cx| {
                        if *hovered {
                            this.menu.selected = Some(index);
                            cx.notify();
                        }
                    }))
                    .on_click(cx.listener(move |this, event: &ClickEvent, window, cx| {
                        if event.modifiers().secondary() {
                            this.toggle_listed_device(index, window, cx);
                        } else {
                            this.choose_device(index, window, cx);
                        }
                    })),
            );
        }
        // Workspaces still being added: progress only, not selectable rows,
        // so keyboard navigation keeps its indices.
        #[cfg(feature = "cloud")]
        {
            view = self.cloud_job_rows(view);
        }
        // The card stays put above the list, which scrolls on its own.
        div()
            .flex()
            .flex_col()
            .child(self.render_activity_card(cx))
            .child(view)
    }

    /// Machines still being added: progress only, not selectable rows, so
    /// keyboard navigation keeps its indices.
    #[cfg(feature = "cloud")]
    fn cloud_job_rows(&self, mut view: Stateful<Div>) -> Stateful<Div> {
        if !self.cloud_jobs.is_empty() {
            view = view.child(
                div()
                    .p(px(8.))
                    .text_color(rgb(self.theme.muted))
                    .child(fl!("devices-adding")),
            );
        }
        for (index, provision) in self.cloud_jobs.iter().enumerate() {
            view = view.child(
                div()
                    .debug_selector(move || format!("device-adding-{index}"))
                    .p(px(8.))
                    .flex_none()
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .child(div().truncate().child(provision.name.clone()))
                            .child(
                                div()
                                    .truncate()
                                    .text_size(px(self.config.ui.size * 0.85))
                                    .text_color(rgb(self.theme.muted))
                                    .child(format!(
                                        "{} · {}",
                                        crate::cloud::name(provision.provider),
                                        provision.status
                                    )),
                            ),
                    )
                    .child(
                        div()
                            .size(px(7.))
                            .flex_none()
                            .rounded_full()
                            .bg(colors::accent(&self.theme)),
                    ),
            );
        }
        view
    }
}
