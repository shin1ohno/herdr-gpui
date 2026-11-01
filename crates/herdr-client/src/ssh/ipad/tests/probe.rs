use super::*;

#[test]
fn outdated_herdr_with_core_keeps_both_registration_choices() {
    assert_eq!(
        with_core_probe(HostProbe::Outdated, || Ok(true)),
        HostProbe::OutdatedCoreAvailable
    );
}

#[test]
fn outdated_herdr_without_core_still_requires_an_update() {
    assert_eq!(
        with_core_probe(HostProbe::Outdated, || Ok(false)),
        HostProbe::Outdated
    );
}

#[test]
fn missing_herdr_with_core_offers_core_registration() {
    assert_eq!(
        with_core_probe(HostProbe::Missing, || Ok(true)),
        HostProbe::CoreAvailable
    );
    assert_eq!(
        with_core_probe(HostProbe::Missing, || Ok(false)),
        HostProbe::Missing
    );
}

#[test]
fn a_compatible_herdr_never_probes_or_switches_to_core() {
    for probe in [HostProbe::Running, HostProbe::Stopped] {
        assert_eq!(
            with_core_probe(probe, || panic!("compatible Herdr needs no core probe")),
            probe
        );
    }
}

#[test]
fn ssh_failure_never_attempts_another_connection() {
    let probe = HostProbe::SshFailed(SshFailure::Auth);
    assert_eq!(
        with_core_probe(probe, || panic!(
            "SSH failed before discovering executables"
        )),
        probe
    );
}

#[test]
fn a_failed_core_probe_keeps_the_herdr_1_advice() {
    for probe in [HostProbe::Outdated, HostProbe::Missing] {
        assert_eq!(
            with_core_probe(probe, || Err(crate::ssh::Error::Timeout.into())),
            probe
        );
    }
}
