use super::*;

#[gpui::test]
fn expected_keys_are_cleared_on_target_change_and_stale_scans_are_ignored(cx: &mut TestAppContext) {
    let (view, cx) = cx.add_window_view(fixture_window);
    open(&view, cx);
    view.update(cx, |view, cx| {
        paste(view, &payload(DIGEST), cx);
        let request = setup::Request::new("dev@computer.local", "", "").unwrap();
        let form = view.menu.device_setup.as_mut().unwrap();
        form.step = Step::FetchingKey(request);
        form.claim = Some(setup::Claim::fixture("host-key-code-target-change"));
        form.task = Some(cx.spawn(async move |_, _| std::future::pending().await));
        target(view, "dev@other.local", cx);
    });
    cx.run_until_parked();
    view.update(cx, |view, cx| {
        assert!(!view.has_device_host_key_code("dev@computer.local"));
        assert!(!view.has_device_host_key_code("dev@other.local"));
        assert!(matches!(form(view).step, Step::Form));
        assert!(form(view).claim.is_none());
        assert!(form(view).task.is_none());
        view.host_key_fetched(Ok(HostKeyScan::Refused(SshFailure::HostKey)), cx);
        assert!(form(view).error.is_none());
        assert!(matches!(form(view).step, Step::Form));
    });
}

#[gpui::test]
fn a_code_never_persists_across_cancel_close_or_finished_attempts(cx: &mut TestAppContext) {
    let (view, cx) = cx.add_window_view(fixture_window);
    open(&view, cx);
    view.update_in(cx, |view, window, cx| {
        let config = format!("{:?}", view.config);
        paste(view, &payload(DIGEST), cx);
        assert_eq!(format!("{:?}", view.config), config);
        view.cancel_device_step(cx);
        assert!(form(view).host_key_code.expected.is_none());
        paste(view, &payload(DIGEST), cx);
        view.device_save_succeeded(setup::SaveOutcome::ExistingServer, cx);
        assert!(form(view).host_key_code.expected.is_none());
        assert!(matches!(form(view).step, Step::SavedWarning(_)));
        view.open_add_device(window, cx);
        view.menu.device_setup.as_mut().unwrap().discovery = None;
        target(view, "dev@computer.local", cx);
        paste(view, &payload(DIGEST), cx);
        view.dismiss_menu(window, cx);
        assert!(view.menu.device_setup.is_none());
        view.open_add_device(window, cx);
        view.menu.device_setup.as_mut().unwrap().discovery = None;
        assert!(form(view).host_key_code.expected.is_none());
        assert!(!form(view).host_key_code.needs_confirmation());
        assert!(
            form(view)
                .host_key_code
                .input
                .as_ref()
                .unwrap()
                .read(cx)
                .text()
                .is_empty()
        );
        assert_eq!(format!("{:?}", view.config), config);
    });
}

#[gpui::test]
fn a_supplied_code_refuses_an_existing_pin_without_a_trust_action(cx: &mut TestAppContext) {
    let (view, cx) = cx.add_window_view(fixture_window);
    open(&view, cx);
    view.update(cx, |view, cx| {
        for scan in [
            HostKeyScan::Trusted,
            HostKeyScan::Refused(SshFailure::HostKey),
        ] {
            paste(view, &payload(DIGEST), cx);
            let request = setup::Request::new("dev@computer.local", "", "").unwrap();
            view.menu.device_setup.as_mut().unwrap().step = Step::FetchingKey(request);
            view.host_key_fetched(Ok(scan), cx);
            assert!(matches!(form(view).step, Step::KeyCodeRefused));
            assert!(form(view).host_key_code.expected.is_none());
            assert!(form(view).error.is_some());
            assert!(form(view).task.is_none());
            view.trust_host_key(cx);
            assert!(matches!(form(view).step, Step::KeyCodeRefused));
            view.cancel_device_step(cx);
        }
    });
}
