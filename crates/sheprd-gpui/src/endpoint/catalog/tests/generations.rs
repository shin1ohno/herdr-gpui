use super::*;
use sheprd_backend::Generation;
use std::{cell::RefCell, sync::Arc};

const PROFILE: &str = "0123456789abcdef0123456789abcdef";
const ENDPOINT: &str = "ssh:0123456789abcdef0123456789abcdef";

fn core_choices() -> Generations {
    Generations::from([(PROFILE.into(), Generation::Herdr2)])
}

fn core_catalog() -> CatalogUpdate {
    load_with(
        || Ok((vec![host(PROFILE, true)], Some(Some(ENDPOINT.into())))),
        || Ok(core_choices()),
    )
    .unwrap()
}

#[test]
fn hosts_are_sampled_before_the_settings_that_publish_them() {
    // Registration finishes while the host read is pending. A settings read
    // started first would capture the old map and misclassify the new host.
    let saved = RefCell::new(Generations::new());
    let update = load_with(
        || {
            *saved.borrow_mut() = core_choices();
            Ok((vec![host(PROFILE, true)], None))
        },
        || Ok(saved.borrow().clone()),
    )
    .unwrap();
    let mut catalog = Catalog::new(&ConnectTarget::Local);
    catalog.accept(&update);
    assert!(update.offers(ENDPOINT));
    assert_eq!(catalog.backend_generation(ENDPOINT), Generation::Herdr2);
    assert_eq!(catalog.backend_generation(LOCAL), Generation::Herdr1);
    assert_eq!(
        catalog.backend_generation("ssh:invalid"),
        Generation::Herdr1
    );
}

#[test]
fn an_unreadable_or_invalid_settings_snapshot_publishes_no_hosts() {
    let read = || Ok((vec![host(PROFILE, true)], None));
    let unreadable = load_with(read, || {
        Err(std::io::Error::from(std::io::ErrorKind::PermissionDenied).into())
    });
    assert!(matches!(unreadable, Err(Error::Io(error))
        if error.kind() == std::io::ErrorKind::PermissionDenied));
    let invalid = load_with(read, || {
        toml::from_str::<toml::Value>("[devices =")?;
        Ok(core_choices())
    });
    assert!(matches!(invalid, Err(Error::Toml(_))));
}

#[test]
fn backend_choices_do_not_resolve_appearance() {
    let mut config = crate::config::Config {
        theme: "/missing/theme".into(),
        ..Default::default()
    };
    config.devices.entry(PROFILE.into()).or_default().generation = Some(Generation::Herdr2);
    assert_eq!(configured_generations(config), core_choices());
}

#[gpui::test]
fn a_first_catalog_in_another_scene_suppresses_gen1_before_any_settings_reload(
    cx: &mut gpui::TestAppContext,
) {
    let update = core_catalog();
    for _ in 0..2 {
        let (view, cx) = cx.add_window_view(crate::sidebar::layout_tests::fixture_window);
        view.update(cx, |view, cx| {
            view.config = crate::config::Config::default();
            view.catalog.accept(&update);
            view.reconcile_catalog(vec![host(PROFILE, true)], Vec::new(), cx);
            // The host runner exercises the same iOS policy explicitly.
            view.apply_catalog_generations(cx);
            let endpoint = &mut view.endpoints[1];
            assert_eq!(endpoint.backend_generation(), Generation::Herdr2);
            let generation = endpoint.generation;
            endpoint.connect(Default::default(), true);
            assert_eq!(endpoint.generation, generation, "no gen1 attempt started");
            assert!(endpoint.connection.handle.is_none());
            view.restore_selection(cx);
            assert_eq!(view.endpoints[view.selected_endpoint].id, ENDPOINT);
            assert!(view.live.snapshot.is_none());
            assert!(view.core_terminals.hub.backend().is_none());

            // A GUI loader sampled before registration can finish afterwards.
            view.config = crate::config::Config::default();
            view.apply_catalog_generations(cx);
            assert_eq!(view.endpoints[1].backend_generation(), Generation::Herdr2);
            assert!(view.endpoints[1].connection.handle.is_none());
        });
    }
}

#[gpui::test]
fn changing_the_selected_backend_retires_its_authoritative_gen1_inbox(
    cx: &mut gpui::TestAppContext,
) {
    let (view, cx) = cx.add_window_view(crate::sidebar::layout_tests::fixture_window);
    view.update(cx, |view, cx| {
        view.reconcile_catalog(vec![host(PROFILE, true)], Vec::new(), cx);
        view.selected_endpoint = 1;
        view.endpoints[1].live = view.live.clone();
        let old = view.endpoints[1].connection.inbox.clone();
        view.catalog.accept(&core_catalog());
        view.apply_catalog_generations(cx);
        assert!(!Arc::ptr_eq(&old, &view.endpoints[1].connection.inbox));
        assert!(view.live.snapshot.is_none());
        assert_eq!(view.selected_generation, view.endpoints[1].generation);
        old.lock().unwrap().status = crate::state::ConnectionStatus::Connected;
        view.endpoints[1].poll(Instant::now());
        assert!(!view.endpoints[1].live.status.is_connected());
        assert!(view.endpoints[1].live.snapshot.is_none());
    });
}
