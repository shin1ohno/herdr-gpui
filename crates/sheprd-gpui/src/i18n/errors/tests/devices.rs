use super::listening_ports::exit_status;
use super::*;
use crate::menu::devices::Error;

#[test]
fn every_devices_variant_has_a_message() {
    let detail = || "machine add: refused".to_owned();
    assert_localized(&[
        Error::Adding,
        Error::Exists("Build box".into()),
        Error::InvalidLabel,
        Error::InvalidSession,
        Error::InvalidTarget,
        Error::NonUtf8SetupPath,
        Error::NoProfile,
        Error::NoStateDirectory,
        Error::Remove {
            status: exit_status(),
            detail: String::new(),
        },
        Error::Remove {
            status: exit_status(),
            detail: detail(),
        },
        Error::Rename {
            status: exit_status(),
            detail: String::new(),
        },
        Error::Rename {
            status: exit_status(),
            detail: detail(),
        },
        Error::ServerRestart,
        Error::Setup {
            status: exit_status(),
            detail: String::new(),
        },
        Error::Setup {
            status: exit_status(),
            detail: detail(),
        },
        Error::SetupTimeout,
        Error::TailscaleJson(serde_json::from_str::<u8>("x").unwrap_err()),
        Error::TailscaleTimeout,
        Error::TerminalSetupUnavailable,
    ]);
}

// Bonjour's `DnsSd` carries a status only its own module builds, so its
// message is checked by the compiler and `just check-i18n` only.
#[cfg(target_os = "ios")]
#[test]
fn every_ipad_devices_variant_has_a_message() {
    assert_localized(&[Error::Bonjour(
        crate::menu::devices::discover::bonjour::Error::Poll(std::io::Error::other("no poll")),
    )]);
}
