//! Local Network permission in the Add Device dialog. macOS refuses the
//! first local network operation while its alert waits for an answer, and
//! there is no API to ask or read the permission (Apple's TN3179). So when a
//! host check is refused, the dialog keeps the host claimed and waits for the
//! user, then checks the host again; nothing has to be restarted.

use super::{add_device::Step, setup};
use crate::HerdrWindow;
use crate::i18n::fl;
use gpui::{prelude::*, *};
use herdr_client::{HostProbe, LocalNetworkAccess, SshFailure};
use std::{
    sync::{Arc, Weak},
    time::Duration,
};

fn settings_url() -> Option<&'static str> {
    if cfg!(target_os = "ios") {
        Some("app-settings:") // i18n: data
    } else if cfg!(target_os = "macos") {
        Some("x-apple.systempreferences:com.apple.preference.security?Privacy_LocalNetwork") // i18n: data
    } else {
        None
    }
}
/// How long a refused host check waits for the user to allow Herdr.
const PERMISSION_WAIT: Duration = Duration::from_secs(60);

impl HerdrWindow {
    /// Opens this app's permission settings on iPadOS, or Local Network on macOS.
    pub(super) fn local_network_settings_button(
        &self,
        id: &'static str,
        cx: &mut Context<Self>,
    ) -> Option<Stateful<Div>> {
        let url = settings_url()?;
        let theme = &self.theme;
        Some(
            div()
                .id(id)
                .debug_selector(move || id.into())
                .flex_none()
                .px_2()
                .py_1()
                .cursor_pointer()
                .rounded(px(crate::config::corners::CONTROL))
                .hover(|s| s.bg(rgb(theme.active)))
                .child(if cfg!(target_os = "ios") {
                    fl!("add-device-ipad-local-network-settings")
                } else {
                    fl!("add-device-local-network-settings")
                })
                .on_click(cx.listener(move |_, _, _, cx| cx.open_url(url))),
        )
    }

    /// The host check was refused the local network. Wait for the user to
    /// allow Herdr, then check the host again with the same claim. Asking
    /// from this long-lived app is also what keeps the system alert up: the
    /// `ssh` that was refused exits too soon to show it. Cancelling drops the
    /// task, which ends the wait and releases the claim.
    pub(super) fn await_local_network(
        &mut self,
        request: setup::Request,
        claim: setup::Claim,
        cx: &mut Context<Self>,
    ) {
        self.await_local_network_with(
            request,
            claim,
            |request, watched| {
                let access = herdr_client::wait_for_local_network(PERMISSION_WAIT, || {
                    watched.strong_count() > 0
                });
                match access {
                    LocalNetworkAccess::Denied => {
                        Ok(HostProbe::SshFailed(SshFailure::LocalNetworkDenied))
                    }
                    LocalNetworkAccess::Allowed | LocalNetworkAccess::Unknown => request.probe(),
                }
            },
            cx,
        );
    }

    pub(super) fn await_local_network_with(
        &mut self,
        request: setup::Request,
        claim: setup::Claim,
        probe: impl FnOnce(&setup::Request, &Weak<()>) -> crate::Result<HostProbe> + Send + 'static,
        cx: &mut Context<Self>,
    ) {
        let Some(form) = &mut self.menu.device_setup else {
            return;
        };
        tracing::info!(
            category = "device_setup",
            "Waiting for Local Network permission"
        );
        form.step = Step::AwaitingLocalNetwork(request.clone());
        form.error = None;
        let alive = Arc::new(());
        let watched = Arc::downgrade(&alive);
        let background = cx.background_executor().spawn(async move {
            let probe = probe(&request, &watched);
            Ok::<_, crate::Error>((claim, probe))
        });
        form.task = Some(cx.spawn(async move |this, cx| {
            let result = background.await;
            drop(alive);
            let _ = this.update(cx, |this, cx| this.device_probed(result, cx));
        }));
        cx.notify();
    }
}

/// What the dialog says while it waits.
pub(super) fn waiting_status(target: &str) -> String {
    if cfg!(target_os = "ios") {
        fl!("add-device-ipad-local-network-waiting", target = target)
    } else {
        fl!("add-device-local-network-waiting", target = target)
    }
}

#[cfg(test)]
mod tests;
