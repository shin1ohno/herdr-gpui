//! The iPad host-key prompt names app storage without exposing its container.

use crate::HerdrWindow;
use crate::i18n::fl;
use gpui::{prelude::*, *};
use herdr_client::HostKeyFingerprint;

impl HerdrWindow {
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
                            .text_color(rgb(theme.muted))
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
                    .text_color(rgb(theme.muted))
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
