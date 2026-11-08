//! The iPad host-key prompt names app storage without exposing its container.

use crate::SheprdWindow;
use crate::i18n::fl;
use gpui::{prelude::*, *};
use sheprd_client::HostKeyFingerprint;

/// Keep renderer choice, internal actions, and footer removal in agreement.
pub(super) fn ipad_host_key() -> bool {
    cfg!(target_os = "ios") || crate::ipad_ui::layout()
}

impl SheprdWindow {
    pub(super) fn render_ipad_host_key_card(
        &self,
        target: &str,
        keys: &[HostKeyFingerprint],
        cx: &mut Context<Self>,
    ) -> Div {
        let card = self.render_ipad_host_key(target, keys);
        if !ipad_host_key() {
            return card;
        }
        card.when(crate::ipad_ui::layout(), |card| {
            card.p(px(12.))
                .border_1()
                .border_color(crate::menu::colors::tint(&self.theme, 3))
                .rounded(px(crate::config::corners::CONTROL))
        })
        .child(
            div()
                .debug_selector(|| "device-setup-host-key-actions".into())
                .flex_none()
                .flex()
                .flex_wrap()
                .justify_end()
                .gap(px(8.))
                .child(
                    self.device_action_button(
                        "device-setup-key-cancel",
                        fl!("add-device-cancel"),
                        true,
                        false,
                    )
                    .on_click(cx.listener(|this, _, _, cx| this.cancel_device_step(cx))),
                )
                .child(
                    self.device_action_button(
                        "device-setup-trust",
                        fl!("add-device-trust-key"),
                        true,
                        true,
                    )
                    .on_click(cx.listener(|this, _, _, cx| this.trust_host_key(cx))),
                ),
        )
    }

    pub(super) fn render_ipad_host_key(&self, target: &str, keys: &[HostKeyFingerprint]) -> Div {
        let theme = &self.theme;
        let mut fingerprints = div()
            .debug_selector(|| "device-setup-fingerprints".into())
            .flex_none()
            .min_w_0()
            .w_full()
            .flex()
            .flex_col()
            .gap(px(4.))
            .p(px(8.))
            .rounded(px(crate::config::corners::CONTROL))
            .bg(rgb(theme.active))
            .font_family(self.config.terminal.family.clone());
        for key in keys {
            fingerprints = fingerprints.child(
                div()
                    .flex_none()
                    .min_w_0()
                    .w_full()
                    .flex()
                    .flex_wrap()
                    .gap_x(px(8.))
                    .child(
                        div()
                            .flex_none()
                            .text_color(rgb(if crate::ipad_ui::layout() {
                                theme.subtext()
                            } else {
                                theme.muted
                            }))
                            .child(key_label(key)),
                    )
                    .child(
                        div()
                            .min_w_0()
                            .whitespace_normal()
                            .child(key.fingerprint.clone()),
                    ),
            );
        }
        // Every section keeps its measured height; only the dialog's body
        // scrolls when the keyboard leaves less room for the prompt.
        div()
            .debug_selector(|| "device-setup-host-key".into())
            .flex_none()
            .min_w_0()
            .w_full()
            .whitespace_normal()
            .flex()
            .flex_col()
            .gap(px(8.))
            .child(
                div()
                    .flex_none()
                    .min_w_0()
                    .w_full()
                    .child(fl!("add-device-key-unknown", target = target)),
            )
            .child(fingerprints)
            .child(
                div()
                    .debug_selector(|| "device-setup-host-key-check".into())
                    .flex_none()
                    .min_w_0()
                    .w_full()
                    .whitespace_normal()
                    .text_color(rgb(if crate::ipad_ui::layout() {
                        theme.subtext()
                    } else {
                        theme.muted
                    }))
                    .child(key_check(keys)),
            )
    }
}

/// The key type, with its size only when known: the in-process client
/// reports no size for RSA keys, and "RSA 0" would read as a broken key.
pub(super) fn key_label(key: &HostKeyFingerprint) -> String {
    if key.bits > 0 {
        format!("{} {}", key.kind, key.bits)
    } else {
        key.kind.clone()
    }
}

/// The host file that holds the first offered key. OpenSSH names its files
/// after the key types it generates; any other type falls back to Ed25519,
/// the default, rather than a path that cannot exist.
pub(super) fn key_check(keys: &[HostKeyFingerprint]) -> String {
    let kind = keys
        .first()
        .map(|key| key.kind.to_ascii_lowercase())
        .filter(|kind| matches!(kind.as_str(), "ed25519" | "ecdsa" | "rsa"))
        .unwrap_or_else(|| "ed25519".to_owned());
    let host_file = format!("/etc/ssh/ssh_host_{kind}_key.pub");
    fl!("add-device-ipad-key-check", host_file = host_file)
}

#[cfg(test)]
mod tests;
