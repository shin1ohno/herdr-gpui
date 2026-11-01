//! On iPadOS, resolving a device whose host key was refused (docs/ios.md,
//! "When the user is asked"). The SSH host menu offers Review Host Key even
//! while disconnected, including a Herdr 2 host: it shows the trusted and
//! the offered fingerprints side by side and removes the trusted key only
//! when the user confirms. The offered key is then shown for approval on its
//! own, so a new key is trusted only by a second explicit step. Reconnects
//! never open this; only the user's click does.

use super::HostMenu;
use crate::HerdrWindow;
use crate::i18n::fl;
use gpui::{prelude::*, *};
use herdr_client::{HostKeyChange, HostKeyFingerprint, HostKeyScan, NewHostKey};

/// The review, from the user's click until the menu closes.
pub(super) struct HostKeyReview {
    step: Step,
    /// Held so the check or save is not cancelled.
    _task: Option<Task<()>>,
}

enum Step {
    Checking,
    /// The host offers a key that matches none trusted for it.
    Changed(HostKeyChange),
    /// Nothing is trusted for the host; its key waits for approval.
    Unpinned(NewHostKey),
    Saving,
    /// The host's key is trusted, so the device can connect again.
    Trusted,
    Failed(String),
}

impl HerdrWindow {
    fn set_host_key_step(&mut self, step: Step, task: Option<Task<()>>) {
        if let Some(host) = &mut self.menu.host {
            host.host_key = Some(HostKeyReview { step, _task: task });
        }
    }

    /// Fetches what the host offers against what is trusted for it.
    fn review_host_key(&mut self, cx: &mut Context<Self>) {
        let Some(target) = self.menu.host.as_ref().map(|host| host.target.clone()) else {
            return;
        };
        tracing::info!(category = "host_key", "Reviewing a refused host key");
        let background = cx.background_executor().spawn(async move {
            if let Some(change) = herdr_client::changed_host_key(&target)? {
                return Ok(Step::Changed(change));
            }
            Ok::<_, herdr_client::Error>(match herdr_client::scan_host_key(&target)? {
                HostKeyScan::New(key) => Step::Unpinned(key),
                HostKeyScan::Trusted => Step::Trusted,
                HostKeyScan::Refused(failure) => Step::Failed(failure.to_string()),
            })
        });
        let task = cx.spawn(async move |this, cx| {
            let step = background
                .await
                .unwrap_or_else(|error| Step::Failed(error.to_string()));
            let _ = this.update(cx, |this, cx| this.host_key_step_done(step, cx));
        });
        self.set_host_key_step(Step::Checking, Some(task));
        cx.notify();
    }

    /// Forgets the trusted keys, then offers the host's key for approval.
    fn forget_trusted_key(&mut self, cx: &mut Context<Self>) {
        let Some(host) = &self.menu.host else {
            return;
        };
        let Some(HostKeyReview {
            step: Step::Changed(change),
            ..
        }) = &host.host_key
        else {
            return;
        };
        let change = change.clone();
        tracing::info!(category = "host_key", "Forgetting a trusted host key");
        let background = cx.background_executor().spawn(async move {
            herdr_client::forget_host_key(&change).map(|()| Step::Unpinned(change.offered))
        });
        let task = cx.spawn(async move |this, cx| {
            let step = background
                .await
                .unwrap_or_else(|error| Step::Failed(error.to_string()));
            let _ = this.update(cx, |this, cx| this.host_key_step_done(step, cx));
        });
        self.set_host_key_step(Step::Saving, Some(task));
        cx.notify();
    }

    /// Trusts exactly the key shown, then reconnects the device if it is the
    /// one this window shows.
    fn trust_offered_key(&mut self, cx: &mut Context<Self>) {
        let Some(HostMenu {
            host_key:
                Some(HostKeyReview {
                    step: Step::Unpinned(key),
                    ..
                }),
            ..
        }) = &self.menu.host
        else {
            return;
        };
        let key = key.clone();
        tracing::info!(
            category = "host_key",
            "Trusting a host key from the host menu"
        );
        let background = cx
            .background_executor()
            .spawn(async move { herdr_client::trust_host_key(&key).map(|()| Step::Trusted) });
        let task = cx.spawn(async move |this, cx| {
            let step = background
                .await
                .unwrap_or_else(|error| Step::Failed(error.to_string()));
            let trusted = matches!(step, Step::Trusted);
            let _ = this.update(cx, |this, cx| {
                let id = this.menu.host.as_ref().map(|host| host.id.clone());
                this.host_key_step_done(step, cx);
                let selected = this
                    .endpoints
                    .get(this.selected_endpoint)
                    .is_some_and(|endpoint| Some(&endpoint.id) == id.as_ref());
                if trusted && selected {
                    this.reconnect(cx);
                }
            });
        });
        self.set_host_key_step(Step::Saving, Some(task));
        cx.notify();
    }

    fn host_key_step_done(&mut self, step: Step, cx: &mut Context<Self>) {
        if let Some(HostMenu {
            host_key: Some(review),
            ..
        }) = &mut self.menu.host
        {
            review.step = step;
            review._task = None;
            cx.notify();
        }
    }

    /// The Forget Host Key row, and the review once it started.
    pub(super) fn render_host_key_review(&self, host: &HostMenu, cx: &mut Context<Self>) -> Div {
        let theme = &self.theme;
        let section = div()
            .debug_selector(|| "host-menu-host-key".into())
            .w_0()
            .min_w_full()
            .flex()
            .flex_col()
            .gap(px(8.));
        let Some(review) = &host.host_key else {
            if host.ssh_target().is_none() {
                return section;
            }
            return section.child(
                div()
                    .id("host-menu-forget-host-key")
                    .debug_selector(|| "host-menu-forget-host-key".into())
                    .min_h(px(self.config.ui.line_height() + 12.))
                    .px(px(8.))
                    .flex()
                    .items_center()
                    .cursor_pointer()
                    .text_color(crate::menu::danger(theme))
                    .hover(|row| row.bg(rgb(theme.active)))
                    .child(fl!("host-menu-forget-host-key"))
                    .on_click(cx.listener(|this, _, _, cx| this.review_host_key(cx))),
            );
        };
        let note = |text: String| div().px(px(8.)).text_color(rgb(theme.muted)).child(text);
        let button = |id: &'static str, label: String| {
            div()
                .id(id)
                .debug_selector(move || id.into())
                .mx(px(8.))
                .p(px(6.))
                .rounded(px(crate::config::corners::CONTROL))
                .bg(rgb(theme.active))
                .cursor_pointer()
                .child(label)
        };
        match &review.step {
            Step::Checking => section.child(note(fl!("host-menu-host-key-checking"))),
            Step::Saving => section.child(note(fl!("host-menu-host-key-saving"))),
            Step::Trusted => section.child(note(fl!("host-menu-host-key-trusted"))),
            Step::Failed(error) => section.child(
                div()
                    .px(px(8.))
                    .text_color(crate::menu::danger(theme))
                    .child(fl!("host-menu-host-key-failed", error = error.clone())),
            ),
            Step::Changed(change) => section
                .child(
                    div()
                        .px(px(8.))
                        .text_color(crate::menu::danger(theme))
                        .child(fl!("host-menu-host-key-changed")),
                )
                .child(note(fl!("host-menu-host-key-pinned")))
                .child(self.render_fingerprints(&change.pinned))
                .child(note(fl!("host-menu-host-key-offered")))
                .child(self.render_fingerprints(&change.offered.fingerprints))
                .child(
                    button(
                        "host-menu-host-key-forget",
                        fl!("host-menu-host-key-forget"),
                    )
                    .text_color(crate::menu::danger(theme))
                    .on_click(cx.listener(|this, _, _, cx| this.forget_trusted_key(cx))),
                ),
            Step::Unpinned(key) => section
                .child(note(fl!("host-menu-host-key-unpinned")))
                .child(self.render_fingerprints(&key.fingerprints))
                .child(
                    button("host-menu-host-key-trust", fl!("host-menu-host-key-trust"))
                        .on_click(cx.listener(|this, _, _, cx| this.trust_offered_key(cx))),
                ),
        }
    }

    fn render_fingerprints(&self, keys: &[HostKeyFingerprint]) -> Div {
        let theme = &self.theme;
        let mut list = div()
            .mx(px(8.))
            .p(px(8.))
            .flex()
            .flex_col()
            .gap(px(4.))
            .rounded(px(crate::config::corners::CONTROL))
            .bg(rgb(theme.active))
            .font_family(self.config.terminal.family.clone());
        for key in keys {
            // The in-process client reports no size for an RSA key.
            let kind = if key.bits > 0 {
                format!("{} {}", key.kind, key.bits) // i18n: data
            } else {
                key.kind.clone()
            };
            list = list.child(
                div()
                    .flex()
                    .flex_wrap()
                    .gap_x(px(8.))
                    .child(div().text_color(rgb(theme.muted)).child(kind))
                    .child(div().min_w_0().child(key.fingerprint.clone())),
            );
        }
        list
    }
}
