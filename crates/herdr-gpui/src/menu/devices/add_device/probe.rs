//! Initial checks and explicit retries share the same claimed request.

use super::{Step, setup};
use crate::HerdrWindow;
use gpui::prelude::*;
use herdr_client::HostProbe;

type ProbeResult = crate::Result<(setup::Claim, crate::Result<HostProbe>)>;

async fn probe_device(request: setup::Request, claim: Option<setup::Claim>) -> ProbeResult {
    let claim = match claim {
        Some(claim) => setup::verify_unsaved(&request, &claim).map(|()| claim),
        None => setup::claim(&request),
    }?;
    Ok((claim, request.probe()))
}

impl HerdrWindow {
    pub(super) fn start_device_probe(
        &mut self,
        request: setup::Request,
        claim: Option<setup::Claim>,
        cx: &mut Context<Self>,
    ) {
        self.start_device_probe_with(request, claim, probe_device, cx);
    }

    pub(super) fn start_device_probe_with<F>(
        &mut self,
        request: setup::Request,
        claim: Option<setup::Claim>,
        probe: impl FnOnce(setup::Request, Option<setup::Claim>) -> F,
        cx: &mut Context<Self>,
    ) where
        F: Future<Output = ProbeResult> + Send + 'static,
    {
        let Some(form) = &mut self.menu.device_setup else {
            return;
        };
        tracing::info!(category = "device_setup", "Checking device");
        form.step = Step::Checking(request.clone());
        form.error = None;
        // Retain a retry's claim so another window cannot start adding the
        // same host between checks. Catalog reads and SSH stay on the worker.
        let background = cx.background_executor().spawn(probe(request, claim));
        form.task = Some(cx.spawn(async move |this, cx| {
            let result = background.await;
            let _ = this.update(cx, |this, cx| this.device_probed(result, cx));
        }));
        cx.notify();
    }

    #[cfg(any(target_os = "ios", test))]
    pub(super) fn recheck_device(&mut self, cx: &mut Context<Self>) {
        self.recheck_device_with(probe_device, cx);
    }

    #[cfg(any(target_os = "ios", test))]
    pub(super) fn recheck_device_with<F>(
        &mut self,
        probe: impl FnOnce(setup::Request, Option<setup::Claim>) -> F,
        cx: &mut Context<Self>,
    ) where
        F: Future<Output = ProbeResult> + Send + 'static,
    {
        let Some(form) = &mut self.menu.device_setup else {
            return;
        };
        let Step::Confirm(request, _) = &form.step else {
            return;
        };
        let request = request.clone();
        let claim = form.claim.take();
        self.start_device_probe_with(request, claim, probe, cx);
    }
}
