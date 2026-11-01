//! Messages the window shows only on iPadOS, where `Display` takes other
//! words than on the host running these tests. Each is held to the iPadOS
//! text by name, so the two cannot drift while only desktop tests run.
use super::*;
use crate::i18n::fl;

/// `shown` reads `english` in English and is a whole message under `en-XA`.
fn assert_ipad_message(shown: impl Fn() -> String, english: &str) {
    crate::i18n::show(&Locale::FALLBACK);
    assert_eq!(shown().replace(ISOLATION, ""), english);
    crate::i18n::show(&Pseudo::Accented.locale());
    let pseudo = shown();
    crate::i18n::show(&Locale::FALLBACK);
    assert!(herdr_i18n::pseudo::is_marked(&pseudo), "{pseudo}");
}

#[test]
fn ipad_local_network_messages_match_their_ipados_text() {
    assert_ipad_message(
        || fl!("error-local-network-denied-ipad"),
        crate::Error::IPADOS_LOCAL_NETWORK_DENIED,
    );
    assert_ipad_message(
        || fl!("error-client-ssh-failure-local-network-denied-ipad"),
        herdr_client::SshFailure::IPADOS_LOCAL_NETWORK_DENIED,
    );
}

#[test]
fn ipad_terminal_setup_message_matches_its_ipados_text() {
    assert_ipad_message(
        || fl!("error-devices-setup-ipad"),
        crate::menu::devices::Error::IPADOS_TERMINAL_SETUP_UNAVAILABLE,
    );
}

#[cfg(feature = "cloud")]
#[test]
fn ipad_cloud_unavailable_message_matches_its_ipados_text() {
    assert_ipad_message(
        || fl!("error-remote-cloud-unavailable-ipad"),
        herdr_remote::cloud::UNAVAILABLE_ON_IPADOS,
    );
}
