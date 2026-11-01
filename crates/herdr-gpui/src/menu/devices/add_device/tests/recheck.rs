use super::*;
use herdr_client::HostProbe;
use std::{
    future::{pending, poll_fn},
    sync::{Arc, Mutex},
    task::{Poll, Waker},
};

type ProbeResult = crate::Result<(setup::Claim, crate::Result<HostProbe>)>;

#[derive(Default)]
struct Completion {
    result: Option<ProbeResult>,
    waker: Option<Waker>,
}

fn pending_probe(
    completion: Arc<Mutex<Completion>>,
) -> impl Future<Output = ProbeResult> + Send + 'static {
    poll_fn(move |cx| {
        let mut completion = completion.lock().unwrap();
        if let Some(result) = completion.result.take() {
            Poll::Ready(result)
        } else {
            completion.waker = Some(cx.waker().clone());
            Poll::Pending
        }
    })
}

fn finish_probe(completion: &Mutex<Completion>, result: ProbeResult) {
    let waker = {
        let mut completion = completion.lock().unwrap();
        completion.result = Some(result);
        completion.waker.take()
    };
    if let Some(waker) = waker {
        waker.wake();
    }
}

#[gpui::test]
fn check_again_moves_the_retained_claim_into_the_task(cx: &mut TestAppContext) {
    let (view, cx) = cx.add_window_view(fixture_window);
    view.update(cx, |view, cx| {
        show_form(view, Step::Form, cx);
        view.recheck_device(cx);
        let form = view.menu.device_setup.as_ref().unwrap();
        assert!(matches!(form.step, Step::Form));
        assert!(form.task.is_none());

        let request = setup::Request::new("test@recheck-retained.test", "Remote", "").unwrap();
        show_form(view, Step::Confirm(request.clone(), Offer::Update), cx);
        let form = view.menu.device_setup.as_mut().unwrap();
        form.claim = Some(setup::Claim::fixture("recheck-retained.test"));
        form.error = Some("Previous check failed".into());

        view.recheck_device_with(
            |current, claim| {
                assert_eq!(current, request);
                let claim = claim.unwrap();
                assert_eq!(claim.user(), "test");
                async move {
                    pending::<()>().await;
                    Ok((claim, Ok(HostProbe::Missing)))
                }
            },
            cx,
        );

        let form = view.menu.device_setup.as_ref().unwrap();
        assert!(matches!(&form.step, Step::Checking(current) if current == &request));
        assert!(form.claim.is_none());
        assert!(form.task.is_some());
        assert!(form.error.is_none());
        view.cancel_device_step(cx);
    });
    cx.run_until_parked();
    view.read_with(cx, |view, _| {
        let form = view.menu.device_setup.as_ref().unwrap();
        assert!(matches!(form.step, Step::Form));
        assert!(form.task.is_none());
    });
}

#[gpui::test]
fn check_again_refuses_a_device_saved_by_another_window(cx: &mut TestAppContext) {
    let (view, cx) = cx.add_window_view(fixture_window);
    view.update(cx, |view, cx| {
        let request = setup::Request::new("test@recheck-duplicate.test", "Remote", "").unwrap();
        show_form(view, Step::Confirm(request.clone(), Offer::Update), cx);
        view.menu.device_setup.as_mut().unwrap().claim =
            Some(setup::Claim::fixture("recheck-duplicate.test"));
        view.recheck_device_with(
            move |current, claim| async move {
                assert_eq!(current, request);
                let claim = claim.unwrap();
                assert_eq!(claim.user(), "test");
                // verify_unsaved refuses the retained claim when another
                // window has saved this host since the previous check.
                Err(super::super::super::Error::Exists("Other window".into()).into())
            },
            cx,
        );
        let form = view.menu.device_setup.as_ref().unwrap();
        assert!(matches!(form.step, Step::Checking(_)));
        assert!(form.claim.is_none());
        assert!(form.task.is_some());
    });
    cx.run_until_parked();
    view.read_with(cx, |view, _| {
        let form = view.menu.device_setup.as_ref().unwrap();
        assert!(matches!(form.step, Step::Form));
        assert!(form.claim.is_none());
        assert_eq!(
            form.error.as_deref(),
            Some("This host and session are already saved as \u{201c}Other window\u{201d}.")
        );
    });
}

#[gpui::test]
fn check_again_ignores_a_stale_task_after_a_replacement_request(cx: &mut TestAppContext) {
    let (view, cx) = cx.add_window_view(fixture_window);
    let stale = Arc::new(Mutex::new(Completion::default()));
    let replacement = Arc::new(Mutex::new(Completion::default()));
    let request = setup::Request::new("test@recheck-replacement.test", "Replacement", "").unwrap();
    view.update(cx, |view, cx| {
        let request = setup::Request::new("test@recheck-stale.test", "Old", "").unwrap();
        show_form(view, Step::Confirm(request, Offer::Update), cx);
        view.menu.device_setup.as_mut().unwrap().claim =
            Some(setup::Claim::fixture("recheck-stale.test"));
        let stale = stale.clone();
        view.recheck_device_with(
            move |_, claim| async move {
                let _claim = claim.unwrap();
                pending_probe(stale).await
            },
            cx,
        );
    });
    cx.run_until_parked();
    assert!(stale.lock().unwrap().waker.is_some());

    view.update(cx, |view, cx| {
        let replacement = replacement.clone();
        view.start_device_probe_with(
            request.clone(),
            None,
            move |_, _| pending_probe(replacement),
            cx,
        );
    });
    cx.run_until_parked();
    // Even a duplicate refusal from the cancelled task must not reset the
    // replacement's Checking step to Form.
    finish_probe(
        &stale,
        Err(super::super::super::Error::Exists("Stale device".into()).into()),
    );
    cx.run_until_parked();
    view.read_with(cx, |view, _| {
        let form = view.menu.device_setup.as_ref().unwrap();
        assert!(matches!(&form.step, Step::Checking(current) if current == &request));
        assert!(form.error.is_none());
        assert!(form.claim.is_none());
        assert!(form.task.is_some());
    });

    finish_probe(
        &replacement,
        Ok((
            setup::Claim::fixture("recheck-replacement.test"),
            Ok(HostProbe::Missing),
        )),
    );
    cx.run_until_parked();
    view.read_with(cx, |view, _| {
        let form = view.menu.device_setup.as_ref().unwrap();
        assert!(
            matches!(&form.step, Step::Confirm(current, Offer::Install) if current == &request)
        );
        assert!(form.error.is_none());
        assert!(form.claim.is_some());
    });
}
