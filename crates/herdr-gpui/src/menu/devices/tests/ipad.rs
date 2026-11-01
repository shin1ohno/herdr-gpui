use super::*;

#[test]
fn ipad_setup_explains_the_host_side_steps_without_offering_a_local_terminal() {
    for (offer, expected) in [
        (Offer::Install, "Install or update Herdr"),
        (Offer::Update, "Install or update Herdr"),
        (Offer::InstallKey, "authorized_keys"),
        (Offer::Terminal, "cannot open a local setup terminal"),
        (Offer::Restart, "cannot open a local setup terminal"),
    ] {
        let text = offer.ipad_instructions("user@host");
        assert!(text.contains(expected), "{text}");
        assert!(text.contains("user@host"), "{text}");
        assert!(!text.contains("ssh-copy-id"), "{text}");
    }
}

#[cfg(target_os = "ios")]
#[gpui::test]
fn a_core_only_host_requires_explicit_registration_and_applies_its_generation_immediately(
    cx: &mut gpui::TestAppContext,
) {
    let (view, cx) = cx.add_window_view(fixture_window);
    cx.update(|_, cx| {
        view.update(cx, |view, cx| {
            let request = setup::Request::new("user@core-only", "Core", "default").unwrap();
            open_form(view, Step::Checking(request), cx);
            view.device_probed(
                Ok((
                    setup::Claim::fixture("core-only-explicit"),
                    Ok(HostProbe::CoreAvailable),
                )),
                cx,
            );
            assert!(matches!(step(view), Step::Confirm(_, Offer::CoreAvailable)));
            assert!(view.menu.device_setup.as_ref().unwrap().task.is_none());
            assert!(view.config.devices.is_empty());
            let profile = "0123456789abcdef0123456789abcdef";
            view.device_save_succeeded(setup::SaveOutcome::CoreAvailable(profile.into()), cx);
            assert_eq!(
                view.config.devices[profile].generation,
                Some(herdr_backend::Generation::Herdr2)
            );
            assert!(matches!(
                step(view),
                Step::SavedWarning("devices-setup-core-available")
            ));
        });
    });
}
