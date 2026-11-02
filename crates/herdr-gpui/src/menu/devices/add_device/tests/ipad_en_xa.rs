#![cfg(not(target_os = "ios"))]

use super::*;
use crate::i18n::leak_tests::{assert_translated, draw, under_en_xa};

#[test]
fn ipad_sheet_and_host_key_card_show_translated_messages() {
    under_en_xa("iPad Add Device sheet", |cx, recorder| {
        let _ipad = crate::ipad_ui::ipad_layout();
        let (view, cx) = cx.add_window_view(fixture_window);
        cx.simulate_resize(size(px(1194.), px(834.)));
        draw(cx);
        recorder.take();
        view.update_in(cx, |view, window, cx| view.open_add_device(window, cx));
        cx.run_until_parked();
        cx.executor()
            .advance_clock(std::time::Duration::from_millis(300));
        cx.run_until_parked();
        draw(cx);
        assert_translated(
            "iPad Add Device sheet",
            &recorder.take(),
            &["default".into()],
        );
        view.update(cx, |view, cx| {
            let key = herdr_client::NewHostKey::fixture_for_tests(
                std::path::PathBuf::from("unused"),
                vec![herdr_client::HostKeyFingerprint {
                    kind: "ED25519".into(),
                    bits: 256,
                    fingerprint: "SHA256:example".into(),
                }],
            );
            view.menu.device_setup.as_mut().unwrap().step = Step::TrustKey(
                setup::Request::new("dev@sheet.test", "Remote", "").unwrap(),
                key,
            );
            cx.notify();
        });
        draw(cx);
        assert_translated(
            "iPad host key card",
            &recorder.take(),
            &["ED25519 256".into(), "SHA256:example".into()],
        );
    });
}
