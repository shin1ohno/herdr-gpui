use super::*;
use std::path::Path;

fn key(path: &Path, digest: &str) -> NewHostKey {
    NewHostKey::fixture_for_tests(path.to_owned(), vec![fingerprint(digest)])
}

#[gpui::test]
fn a_matching_scan_uses_the_existing_trust_path_without_the_tofu_prompt(cx: &mut TestAppContext) {
    let scratch = tempfile::tempdir().unwrap();
    let pin = scratch.path().join("known_hosts");
    let (view, cx) = cx.add_window_view(fixture_window);
    open(&view, cx);
    view.update(cx, |view, cx| {
        paste(view, &payload(DIGEST), cx);
        let request = setup::Request::new("dev@computer.local", "", "").unwrap();
        let setup = view.menu.device_setup.as_mut().unwrap();
        setup.step = Step::FetchingKey(request);
        setup.claim = Some(setup::Claim::fixture("host-key-code-match"));
        view.host_key_fetched(Ok(HostKeyScan::New(key(&pin, DIGEST))), cx);
        assert!(matches!(form(view).step, Step::Checking(_)));
        assert!(form(view).host_key_code.matched);
        assert!(form(view).task.is_some());
        // The background trust has deliberately not been polled. Cancel it
        // before the test dispatcher can perform any I/O.
        view.cancel_device_step(cx);
        assert!(form(view).task.is_none());
        assert!(!pin.exists());
    });
}

#[gpui::test]
fn a_mismatching_scan_never_offers_or_schedules_trust(cx: &mut TestAppContext) {
    let scratch = tempfile::tempdir().unwrap();
    let pin = scratch.path().join("known_hosts");
    let (view, cx) = cx.add_window_view(fixture_window);
    open(&view, cx);
    view.update(cx, |view, cx| {
        paste(view, &payload(DIGEST), cx);
        let request = setup::Request::new("dev@computer.local", "", "").unwrap();
        view.menu.device_setup.as_mut().unwrap().step = Step::FetchingKey(request);
        view.host_key_fetched(Ok(HostKeyScan::New(key(&pin, OTHER_DIGEST))), cx);
        assert!(matches!(form(view).step, Step::KeyCodeRefused));
        assert!(form(view).host_key_code.expected.is_none());
        let error = form(view).error.as_ref().unwrap();
        assert!(error.contains(DIGEST));
        assert!(error.contains(OTHER_DIGEST));
        assert!(form(view).task.is_none());
        assert!(!form(view).host_key_code.matched);
        view.trust_host_key(cx);
        assert!(matches!(form(view).step, Step::KeyCodeRefused));
        assert!(form(view).task.is_none());
        assert!(!pin.exists());
    });
    cx.update(|window, cx| window.draw(cx).clear(cx));
    assert!(cx.debug_bounds("device-setup-trust").is_none());
    assert!(cx.debug_bounds("device-setup-cancel").is_some());
}

#[gpui::test]
fn without_a_code_the_existing_tofu_flow_is_unchanged(cx: &mut TestAppContext) {
    let scratch = tempfile::tempdir().unwrap();
    let pin = scratch.path().join("known_hosts");
    let (view, cx) = cx.add_window_view(fixture_window);
    open(&view, cx);
    view.update(cx, |view, cx| {
        let request = setup::Request::new("dev@computer.local", "", "").unwrap();
        view.menu.device_setup.as_mut().unwrap().step = Step::FetchingKey(request);
        view.host_key_fetched(Ok(HostKeyScan::New(key(&pin, DIGEST))), cx);
        assert!(matches!(form(view).step, Step::TrustKey(..)));
        assert!(form(view).task.is_none());
        assert!(!pin.exists());
    });
}

#[gpui::test]
fn target_edits_fence_a_scan_even_before_the_change_subscription_runs(cx: &mut TestAppContext) {
    let scratch = tempfile::tempdir().unwrap();
    let pin = scratch.path().join("known_hosts");
    let (view, cx) = cx.add_window_view(fixture_window);
    open(&view, cx);
    view.update(cx, |view, cx| {
        paste(view, &payload(DIGEST), cx);
        let request = setup::Request::new("dev@computer.local", "", "").unwrap();
        view.menu.device_setup.as_mut().unwrap().step = Step::FetchingKey(request);
        target(view, "dev@other.local", cx);
        view.host_key_fetched(Ok(HostKeyScan::New(key(&pin, DIGEST))), cx);
        assert!(matches!(form(view).step, Step::Form));
        assert!(form(view).host_key_code.expected.is_none());
        assert!(form(view).task.is_none());
        assert!(!pin.exists());
    });
}
