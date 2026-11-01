use super::*;

#[test]
fn outdated_core_offer_keeps_the_herdr_1_update_and_recheck_path() {
    crate::i18n::show(&herdr_i18n::Locale::FALLBACK);
    let text = Offer::OutdatedCoreAvailable.ipad_instructions("user@host");
    for expected in [
        "Herdr 1",
        "user@host",
        "too old",
        "Herdr 2 is installed",
        "Add it as a Herdr 2 host",
        "update Herdr 1 from that computer",
        "Check again",
        "Starting a terminal remains a separate action",
    ] {
        assert!(text.contains(expected), "{expected}: {text}");
    }
}

#[test]
fn core_only_offer_explains_registration_without_herdr_1_update_advice() {
    crate::i18n::show(&herdr_i18n::Locale::FALLBACK);
    let text = Offer::CoreAvailable.ipad_instructions("user@core-only");
    for expected in [
        "Herdr 2 is installed",
        "user@core-only",
        "Add it as a Herdr 2 host",
        "Starting a terminal remains a separate action",
    ] {
        assert!(text.contains(expected), "{expected}: {text}");
    }
    assert!(!text.contains("Herdr 1"), "{text}");
    assert!(!text.contains("update"), "{text}");
}

#[test]
fn both_core_offers_name_the_explicit_registration_action() {
    crate::i18n::show(&herdr_i18n::Locale::FALLBACK);
    for offer in [Offer::CoreAvailable, Offer::OutdatedCoreAvailable] {
        assert_eq!(offer.action(), "Add Herdr 2 host", "{offer:?}");
    }
}
