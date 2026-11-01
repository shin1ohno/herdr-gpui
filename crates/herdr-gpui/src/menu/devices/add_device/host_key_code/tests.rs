#![allow(clippy::unwrap_used)]

use super::*;
use crate::sidebar::layout_tests::fixture_window;
use core::prelude::v1::test;
use gpui::{TestAppContext, VisualTestContext};
#[cfg(not(target_os = "ios"))]
use herdr_client::NewHostKey;
use herdr_client::{HostKeyScan, SshFailure};

mod confirmation;
mod lifecycle;
#[cfg(not(target_os = "ios"))]
mod matching;
mod render;

const DIGEST: &str = "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA";
const OTHER_DIGEST: &str = "BBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBA";

fn payload(digest: &str) -> String {
    format!("herdr-ipad://host-key?v=1&fp=ssh-ed25519:SHA256:{digest}&host=computer.local")
}

fn target(view: &mut HerdrWindow, value: &str, cx: &mut Context<HerdrWindow>) {
    view.menu.device_setup.as_mut().unwrap().fields[0]
        .update(cx, |input, cx| input.set_text_selected(value, cx));
}

fn open(view: &Entity<HerdrWindow>, cx: &mut VisualTestContext) {
    view.update_in(cx, |view, window, cx| {
        view.open_add_device(window, cx);
        view.menu.device_setup.as_mut().unwrap().discovery = None;
        target(view, "dev@computer.local", cx);
    });
    cx.run_until_parked();
}

fn paste(view: &mut HerdrWindow, text: &str, cx: &mut Context<HerdrWindow>) {
    let input = view
        .menu
        .device_setup
        .as_ref()
        .unwrap()
        .host_key_code
        .input
        .clone()
        .unwrap();
    input.update(cx, |input, cx| input.set_text_selected(text, cx));
    view.accept_device_host_key_code(true, cx);
}

fn form(view: &HerdrWindow) -> &Setup {
    view.menu.device_setup.as_ref().unwrap()
}

fn fingerprint(digest: &str) -> HostKeyFingerprint {
    HostKeyFingerprint {
        kind: "ED25519".into(),
        bits: 256,
        fingerprint: format!("SHA256:{digest}"),
    }
}
