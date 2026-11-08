//! The Add Device form and its actions.

use super::{Setup, Step, actions::step_actions, host_key::ipad_host_key, setup};
use crate::{SheprdWindow, i18n::fl};
use gpui::{prelude::*, *};
use sheprd_client::HostProbe;

impl SheprdWindow {
    pub(in crate::menu) fn render_add_device(
        &self,
        compact: bool,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let theme = &self.theme;
        let frame = |pane: Div, cx: &mut Context<Self>| {
            if crate::ipad_ui::layout() {
                self.add_device_sheet(pane, cx)
            } else {
                self.add_device_frame(
                    "device-setup-dialog",
                    "device-setup-close",
                    compact,
                    pane,
                    cx,
                )
                // On iPad the overlay caps the panel to the visible viewport,
                // including keyboard insets, and only the body may overflow that
                // allocation. The desktop dialog keeps its own sizing.
                .when(crate::ipad_ui::active(), |frame| {
                    frame.max_h_full().overflow_hidden()
                })
            }
        };
        let Some(setup) = &self.menu.device_setup else {
            return frame(div(), cx);
        };
        if let Step::SavedWarning(warning) = setup.step {
            return frame(self.render_device_saved_warning(warning, cx), cx);
        }
        let ghost = matches!(setup.step, Step::Form)
            .then(|| {
                let text = setup.fields[0].read(cx).text().to_owned();
                self.device_completion(setup, &text).ghost(&text)
            })
            .flatten();
        setup.fields[0].update(cx, |field, cx| field.set_completion(ghost, cx));
        let mut body = div()
            .id("device-setup-body")
            .debug_selector(|| "device-setup-body".into())
            .flex_1()
            .min_h_0()
            // The body takes the pane's width and wraps its prose. A zero
            // minimum width would also make taffy measure every truncated row
            // at its min-content first, shaping device names cut to a letter.
            .when(crate::ipad_ui::active(), |body| {
                body.w_full().whitespace_normal()
            })
            .overflow_y_scroll()
            .p(px(16.))
            .flex()
            .flex_col()
            .gap(px(12.))
            .child(div().flex_none().text_color(rgb(theme.subtext())).child(
                if cfg!(target_os = "ios") {
                    fl!("add-device-ipad-intro")
                } else {
                    fl!("add-device-intro")
                },
            ));
        if crate::ipad_ui::layout() {
            body = body.child(
                self.device_sheet_field(0, fl!("add-device-target"), &setup.fields[0], cx)
                    .flex_none(),
            );
            if matches!(setup.step, Step::Form)
                && let Some(discovery) = &setup.discovery
            {
                body = body.child(self.render_device_suggestions(setup, discovery, cx));
            }
            body = body.child(
                div()
                    .flex_none()
                    .flex()
                    .gap(px(12.))
                    .child(self.device_sheet_field(
                        1,
                        fl!("add-device-label"),
                        &setup.fields[1],
                        cx,
                    ))
                    .child(self.device_sheet_field(
                        2,
                        fl!("add-device-session"),
                        &setup.fields[2],
                        cx,
                    )),
            );
        } else {
            for (index, (label, field)) in [
                fl!("add-device-target"),
                fl!("add-device-label"),
                fl!("add-device-session"),
            ]
            .into_iter()
            .zip(&setup.fields)
            .enumerate()
            {
                body = body.child(
                    div()
                        .flex_none()
                        .flex()
                        .flex_col()
                        .gap(px(6.))
                        .child(label)
                        .child(field.clone()),
                );
                // Suggestions fill the target, so they sit right under it, and
                // only while the form can still take one.
                if index == 0
                    && matches!(setup.step, Step::Form)
                    && let Some(discovery) = &setup.discovery
                {
                    body = body.child(self.render_device_suggestions(setup, discovery, cx));
                }
            }
        }
        // iPadOS verifies every host key, and its own key must be on the host.
        #[cfg(not(target_os = "ios"))]
        {
            body = body.child(self.render_host_key_choice(setup, cx));
        }
        #[cfg(target_os = "ios")]
        {
            body = body.child(self.render_device_key(setup, cx));
        }
        if crate::ipad_ui::active() {
            body = body.child(self.render_device_host_key_code(setup, cx));
        }
        if let Some(error) = &setup.error {
            body = body.child(
                div()
                    .flex_none()
                    .text_color(crate::menu::danger(theme))
                    .child(error.clone()),
            );
        }
        let status = match &setup.step {
            Step::Form => None,
            Step::Checking(request) => Some(fl!("add-device-checking", target = request.target())),
            Step::AwaitingLocalNetwork(request) => Some(
                super::super::local_network::waiting_status(request.target()),
            ),
            Step::Saving(request, HostProbe::Stopped) => {
                Some(fl!("add-device-starting", target = request.target()))
            }
            Step::Saving(request, _) => Some(fl!("add-device-saving", target = request.target())),
            Step::Saved | Step::SavedWarning(_) => None,
            Step::KeyCodeRefused => None,
            Step::FetchingKey(request) => {
                Some(fl!("add-device-fetching-key", target = request.target()))
            }
            Step::TrustKey(request, key) => {
                body = if ipad_host_key() {
                    body.child(self.render_ipad_host_key_card(
                        request.target(),
                        &key.fingerprints,
                        cx,
                    ))
                } else {
                    body.child(self.render_host_key(request.target(), key))
                };
                None
            }
            Step::Confirm(request, offer) => Some(offer.question(
                request.target(),
                setup.claim.as_ref().map(setup::Claim::user),
            )),
            Step::Verifying(request, _) => {
                Some(fl!("add-device-verifying", target = request.target()))
            }
            Step::Activating(..) | Step::Opening(..) => Some(fl!("add-device-opening")),
        };
        if let Some(status) = status {
            body = body.child(
                div()
                    .debug_selector(|| "device-setup-status".into())
                    .flex_none()
                    .text_color(rgb(if crate::ipad_ui::layout() {
                        theme.subtext()
                    } else {
                        theme.muted
                    }))
                    .child(status),
            );
        }
        frame(
            div()
                .flex_1()
                .min_h_0()
                .when(crate::ipad_ui::active(), |pane| {
                    pane.max_h_full().overflow_hidden()
                })
                .flex()
                .flex_col()
                .child(body)
                .children(self.render_device_setup_footer(setup, cx)),
            cx,
        )
    }

    fn render_device_setup_footer(&self, setup: &Setup, cx: &mut Context<Self>) -> Option<Div> {
        let sheet = crate::ipad_ui::layout();
        // The same predicate chooses the iPad renderer and its internal actions.
        if matches!(setup.step, Step::TrustKey(..)) && ipad_host_key() {
            return None;
        }
        let settings = matches!(setup.step, Step::AwaitingLocalNetwork(_))
            .then(|| self.local_network_settings_button("device-setup-settings", cx))
            .flatten()
            .map(|button| {
                button.when(sheet, |button| {
                    button
                        .min_h(px(crate::ipad_ui::metrics::TOUCH))
                        .flex()
                        .items_center()
                })
            });
        #[cfg(target_os = "ios")]
        let check_again = matches!(setup.step, Step::Confirm(..));
        #[cfg(not(target_os = "ios"))]
        let check_again = false;
        if sheet && settings.is_none() && !check_again {
            return None;
        }
        let actions = step_actions(&setup.step);
        let mut footer = div()
            .debug_selector(|| "device-setup-footer".into())
            .flex_none()
            .p(px(16.))
            .border_t_1()
            .border_color(rgb(self.theme.active))
            .flex()
            .when(crate::ipad_ui::active(), |footer| footer.flex_wrap())
            .justify_end()
            .gap(px(8.))
            .children(settings);
        if actions.cancel && !sheet {
            footer = footer.child(
                self.device_action_button(
                    "device-setup-cancel",
                    fl!("add-device-cancel"),
                    true,
                    false,
                )
                .on_click(cx.listener(|this, _, _, cx| this.cancel_device_step(cx))),
            );
        }
        #[cfg(target_os = "ios")]
        if check_again {
            footer = footer.child(
                self.device_action_button(
                    "device-setup-check-again",
                    fl!("add-device-check-again"),
                    true,
                    false,
                )
                .on_click(cx.listener(|this, _, _, cx| this.recheck_device(cx))),
            );
        }
        if !sheet && let Some(primary) = actions.primary {
            footer = footer.child(self.render_device_primary(primary, primary.id(), setup, cx));
        }
        Some(footer)
    }
}
