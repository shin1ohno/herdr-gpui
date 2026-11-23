//! iPad's in-process SSH. `herdr-ssh` is plain Rust, so its messages are
//! checked on every Unix host, not only where iPadOS builds run tests.
use super::*;
use herdr_ssh::{Error, Failure, forward};
use std::io;

#[test]
fn every_ssh_failure_has_a_message() {
    let failures = [
        Failure::Auth,
        Failure::Cancelled,
        Failure::HostKeyChanged,
        Failure::HostKeyUnknown,
        Failure::Protocol,
        Failure::Timeout,
        Failure::Unreachable,
    ];
    assert_localized(&failures);
    assert_localized(&failures.map(Error::Failure));
}

#[test]
fn every_ssh_error_has_a_message() {
    assert_localized(&[
        Error::InvalidClientKey {
            reason: "not an uncompressed P-256 point",
        },
        Error::InvalidTarget {
            reason: "missing user",
        },
        Error::KnownHosts {
            operation: "is too large",
            source: io::Error::other("disk on fire"),
        },
        Error::PinnedMeanwhile,
        Error::Forward(forward::Error::Prohibited),
    ]);
    assert_verbatim(&[Error::Io(io::Error::other("disk on fire"))]);
}

#[test]
fn every_forward_error_has_a_message() {
    assert_localized(&[
        forward::Error::Busy,
        forward::Error::Connect(Failure::Auth),
        forward::Error::ConnectionLost(Failure::Timeout),
        forward::Error::InvalidPort,
        forward::Error::NotLoopback,
        forward::Error::PortClosed,
        forward::Error::Prohibited,
        forward::Error::Refused,
        forward::Error::Stopped,
        forward::Error::Timeout,
    ]);
    assert_verbatim(&[forward::Error::Io(io::Error::other("disk on fire"))]);
}

#[cfg(target_os = "ios")]
#[test]
fn every_in_process_ssh_variant_has_a_message() {
    use sheprd_client::ssh::in_process::Error;
    assert_localized(&[
        Error::DeviceKey(sheprd_credentials::Error::Policy),
        Error::ProcessUnsupported,
        Error::Ssh(herdr_ssh::Error::PinnedMeanwhile),
        Error::StateDir,
    ]);
}
