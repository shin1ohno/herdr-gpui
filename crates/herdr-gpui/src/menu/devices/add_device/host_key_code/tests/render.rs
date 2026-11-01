use super::*;
use crate::i18n::leak_tests::{assert_translated, draw, under_en_xa};
use gpui::size;

#[gpui::test]
fn url_confirmation_shows_the_warning_and_no_submit_action_is_enabled(cx: &mut TestAppContext) {
    let (view, cx) = cx.add_window_view(fixture_window);
    open(&view, cx);
    cx.simulate_resize(size(px(360.), px(800.)));
    view.update_in(cx, |view, window, cx| {
        view.receive_host_key_code_url(&payload(DIGEST), window, cx)
    });
    cx.update(|window, cx| window.draw(cx).clear(cx));
    for selector in [
        "host-key-code-confirm-warning",
        "host-key-code-accept",
        "host-key-code-dismiss",
    ] {
        assert!(cx.debug_bounds(selector).is_some(), "{selector}");
    }
    assert!(cx.debug_bounds("device-setup-trust").is_none());
}

#[test]
fn a_code_awaiting_confirmation_does_not_label_the_form_as_checking() {
    under_en_xa("host key code pending submit label", |cx, recorder| {
        let (view, cx) = cx.add_window_view(fixture_window);
        open(&view, cx);
        cx.simulate_resize(size(px(1000.), px(900.)));
        view.update_in(cx, |view, window, cx| {
            view.receive_host_key_code_url(&payload(DIGEST), window, cx)
        });
        draw(cx);
        let shown = recorder.take();
        let checking = fl!("add-device-submit-checking");
        let submit = fl!("add-device-submit");
        assert!(!shown.contains(&checking), "{shown:?}");
        assert!(shown.contains(&submit), "{shown:?}");
    });
}

#[test]
fn host_key_code_add_device_copy_is_translated_under_en_xa() {
    under_en_xa("host key code Add Device en-XA", |cx, recorder| {
        let (view, cx) = cx.add_window_view(fixture_window);
        open(&view, cx);
        cx.simulate_resize(size(px(1000.), px(900.)));
        draw(cx);
        recorder.take();
        view.update_in(cx, |view, window, cx| {
            view.receive_host_key_code_url(&payload(DIGEST), window, cx)
        });
        draw(cx);
        assert_translated(
            "URL host key confirmation",
            &recorder.take(),
            &[format!("ssh-ed25519 SHA256:{DIGEST}")],
        );
        view.update(cx, |view, cx| view.accept_device_host_key_code(false, cx));
        draw(cx);
        assert_translated("accepted host key code", &recorder.take(), &[]);
    });
}
