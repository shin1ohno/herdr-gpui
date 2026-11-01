use super::*;

#[gpui::test]
fn a_url_code_requires_explicit_confirmation_and_ignores_replacement_delivery(
    cx: &mut TestAppContext,
) {
    let (view, cx) = cx.add_window_view(fixture_window);
    open(&view, cx);
    view.update_in(cx, |view, window, cx| {
        view.receive_host_key_code_url(&payload(DIGEST), window, cx);
        assert!(form(view).host_key_code.needs_confirmation());
        assert!(!view.has_device_host_key_code("dev@computer.local"));
        view.submit_device_setup(cx);
        assert!(matches!(form(view).step, Step::Form));
        assert!(form(view).task.is_none());
        view.receive_host_key_code_url(&payload(OTHER_DIGEST), window, cx);
        view.accept_device_host_key_code(false, cx);
        assert!(!form(view).host_key_code.needs_confirmation());
        assert_eq!(
            view.device_key_code_matches("dev@computer.local", &[fingerprint(DIGEST)]),
            Some(true)
        );
        view.receive_host_key_code_url(&payload(OTHER_DIGEST), window, cx);
        assert!(!form(view).host_key_code.needs_confirmation());
        assert_eq!(
            view.device_key_code_matches("dev@computer.local", &[fingerprint(DIGEST)]),
            Some(true)
        );
    });
}

#[gpui::test]
fn dismissing_a_url_code_never_creates_an_expectation(cx: &mut TestAppContext) {
    let (view, cx) = cx.add_window_view(fixture_window);
    open(&view, cx);
    view.update_in(cx, |view, window, cx| {
        view.receive_host_key_code_url(&payload(DIGEST), window, cx);
        view.dismiss_device_host_key_code(cx);
        assert!(!form(view).host_key_code.needs_confirmation());
        assert!(!view.has_device_host_key_code("dev@computer.local"));
        view.accept_device_host_key_code(false, cx);
        assert!(!view.has_device_host_key_code("dev@computer.local"));
    });
}

#[gpui::test]
fn a_pasted_code_is_explicit_but_oversized_or_invalid_input_is_refused(cx: &mut TestAppContext) {
    let (view, cx) = cx.add_window_view(fixture_window);
    open(&view, cx);
    view.update(cx, |view, cx| {
        for invalid in [
            "x".repeat(4096),
            format!("{}{}", payload(DIGEST), " ".repeat(4096)),
            "https://example.com/".into(),
        ] {
            paste(view, &invalid, cx);
            assert!(form(view).error.is_some());
            assert!(!view.has_device_host_key_code("dev@computer.local"));
        }
        paste(view, &payload(DIGEST), cx);
        assert!(!form(view).host_key_code.needs_confirmation());
        assert!(form(view).error.is_none());
        assert_eq!(
            view.device_key_code_matches("dev@computer.local", &[fingerprint(DIGEST)]),
            Some(true)
        );
    });
}

#[gpui::test]
fn accepting_a_url_code_requires_a_target_but_pending_input_can_wait_for_typing(
    cx: &mut TestAppContext,
) {
    let (view, cx) = cx.add_window_view(fixture_window);
    view.update_in(cx, |view, window, cx| {
        view.receive_host_key_code_url(&payload(DIGEST), window, cx);
        view.menu.device_setup.as_mut().unwrap().discovery = None;
        view.accept_device_host_key_code(false, cx);
        assert!(form(view).host_key_code.needs_confirmation());
        assert!(form(view).host_key_code.expected.is_none());
        target(view, "dev@computer.local", cx);
    });
    cx.run_until_parked();
    view.update(cx, |view, cx| {
        view.accept_device_host_key_code(false, cx);
        assert!(view.has_device_host_key_code("dev@computer.local"));
    });
}

#[gpui::test]
fn accepting_a_paste_moves_focus_off_the_hidden_field_and_enter_does_not_reparse_it(
    cx: &mut TestAppContext,
) {
    let (view, cx) = cx.add_window_view(fixture_window);
    open(&view, cx);
    view.update_in(cx, |view, window, cx| {
        let input = form(view).host_key_code.input.clone().unwrap();
        input.update(cx, |input, cx| {
            input.set_text_selected(&payload(DIGEST), cx)
        });
        window.focus(&input.read(cx).focus.clone(), cx);
        assert!(view.apply_focused_host_key_code(window, cx));
        assert!(form(view).fields[0].read(cx).focus.is_focused(window));
        assert!(!view.apply_focused_host_key_code(window, cx));
        assert!(form(view).error.is_none());
        assert!(view.has_device_host_key_code("dev@computer.local"));
    });
}
