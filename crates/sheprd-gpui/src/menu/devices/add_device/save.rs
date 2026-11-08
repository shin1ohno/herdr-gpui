//! Saving a probed host, including the explicit choice of a Herdr 2 host.
use super::super::setup;
use super::{Offer, Setup, Step};
use crate::SheprdWindow;
use gpui::prelude::*;
use sheprd_client::HostProbe;

impl SheprdWindow {
    pub(super) fn save_device(
        &mut self,
        request: setup::Request,
        probe: HostProbe,
        claim: setup::Claim,
        cx: &mut Context<Self>,
    ) {
        let Some(form) = &mut self.menu.device_setup else {
            return;
        };
        form.step = Step::Saving(request.clone(), probe);
        // The claim travels with the save and comes back, so a failure that
        // offers a terminal still holds the host.
        let background = cx.background_executor().spawn(async move {
            #[cfg(target_os = "ios")]
            let result = if probe == HostProbe::CoreAvailable {
                setup::save_core(&request, &claim)
            } else {
                setup::save(&request, &claim)
            };
            #[cfg(not(target_os = "ios"))]
            let result = setup::save(&request, &claim);
            (result, claim)
        });
        form.task = Some(cx.spawn(async move |this, cx| {
            let (result, claim) = background.await;
            let _ = this.update(cx, |this, cx| {
                let Some(Setup {
                    step: Step::Saving(request, _),
                    ..
                }) = &this.menu.device_setup
                else {
                    return;
                };
                let request = request.clone();
                match &result {
                    Ok(_) => tracing::info!(category = "device_setup", "Device saved"),
                    Err(_) => tracing::warn!(category = "device_setup", "Device save failed"),
                }
                let error = match result {
                    Ok(outcome) => {
                        this.device_save_succeeded(outcome, cx);
                        return;
                    }
                    Err(error) => error,
                };
                if this.refuse_duplicate(&error, cx) {
                    return;
                }
                this.set_device_error(Some(error.to_string()));
                let offer = match error {
                    crate::Error::Devices(super::super::Error::ServerRestart) => Offer::Restart,
                    _ => Offer::Terminal,
                };
                if let Some(form) = &mut this.menu.device_setup {
                    form.step = Step::Confirm(request, offer);
                    form.claim = Some(claim);
                }
                cx.notify();
            });
        }));
        cx.notify();
    }

    #[cfg(target_os = "ios")]
    pub(super) fn confirm_core_device(&mut self, cx: &mut Context<Self>) {
        let Some(form) = &mut self.menu.device_setup else {
            return;
        };
        let Step::Confirm(request, Offer::CoreAvailable | Offer::OutdatedCoreAvailable) =
            &form.step
        else {
            return;
        };
        let request = request.clone();
        let Some(claim) = form.claim.take() else {
            return;
        };
        self.save_device(request, HostProbe::CoreAvailable, claim, cx);
    }
}
