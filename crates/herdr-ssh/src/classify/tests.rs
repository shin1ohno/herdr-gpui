#![allow(clippy::unwrap_used)]

use russh::{ChannelOpenFailure, Error as E};

use super::*;
use crate::HostKey;
use crate::host_key::tests::ED25519;

fn new_key() -> Offered {
    Offered::New(HostKey::from_known_hosts("ssh-ed25519", ED25519).unwrap())
}

fn changed_key() -> Offered {
    Offered::Changed(HostKey::from_known_hosts("ssh-ed25519", ED25519).unwrap())
}

fn no_common(kind: AlgorithmKind) -> E {
    E::NoCommonAlgo {
        kind,
        ours: vec!["ssh-ed25519".into()],
        theirs: vec!["Host key verification failed".into()],
    }
}

#[test]
fn refused_key_is_unknown_only_without_pins() {
    assert_eq!(
        classify(&E::UnknownKey, Some(&new_key()), false),
        Failure::HostKeyUnknown
    );
    assert_eq!(
        classify(&E::UnknownKey, Some(&changed_key()), true),
        Failure::HostKeyChanged
    );
    assert_eq!(
        classify(&E::UnknownKey, Some(&Offered::Refused), false),
        Failure::HostKeyChanged
    );
    // A refusal whose cause was not recorded fails closed.
    assert_eq!(
        classify(&E::UnknownKey, None, false),
        Failure::HostKeyChanged
    );
}

#[test]
fn host_key_errors() {
    assert_eq!(
        classify(&E::KeyChanged { line: 3 }, None, true),
        Failure::HostKeyChanged
    );
    assert_eq!(
        classify(&E::WrongServerSig, None, false),
        Failure::HostKeyChanged
    );
}

#[test]
fn no_common_host_key_type_is_a_changed_key_only_when_pinned() {
    assert_eq!(
        classify(&no_common(AlgorithmKind::Key), None, true),
        Failure::HostKeyChanged
    );
    assert_eq!(
        classify(&no_common(AlgorithmKind::Key), None, false),
        Failure::Protocol
    );
    assert_eq!(
        classify(&no_common(AlgorithmKind::Cipher), None, true),
        Failure::Protocol
    );
    assert_eq!(
        classify(&no_common(AlgorithmKind::Kex), None, true),
        Failure::Protocol
    );
}

#[test]
fn timeouts() {
    for error in [
        E::ConnectionTimeout,
        E::KeepaliveTimeout,
        E::InactivityTimeout,
    ] {
        assert_eq!(classify(&error, None, true), Failure::Timeout);
    }
    let timed_out = E::IO(io::Error::from(io::ErrorKind::TimedOut));
    assert_eq!(classify(&timed_out, None, true), Failure::Timeout);
}

#[test]
fn unreachable() {
    for kind in [
        io::ErrorKind::ConnectionRefused,
        io::ErrorKind::ConnectionReset,
        io::ErrorKind::NotFound,
        io::ErrorKind::Other,
    ] {
        assert_eq!(
            classify(&E::IO(io::Error::from(kind)), None, true),
            Failure::Unreachable
        );
    }
    for error in [E::HUP, E::Disconnect, E::SendError, E::RecvError] {
        assert_eq!(classify(&error, None, true), Failure::Unreachable);
    }
}

#[test]
fn auth() {
    for error in [
        E::NotAuthenticated,
        E::NoAuthMethod,
        E::UnsupportedAuthMethod,
    ] {
        assert_eq!(classify(&error, None, true), Failure::Auth);
    }
}

#[test]
fn everything_else_is_a_protocol_failure() {
    for error in [
        E::Version,
        E::Kex,
        E::KexInit,
        E::PacketAuth,
        E::Inconsistent,
        E::DecryptionError,
        E::ChannelOpenFailure(ChannelOpenFailure::AdministrativelyProhibited),
        E::RequestDenied,
        E::InvalidConfig("Permission denied (publickey)".into()),
    ] {
        assert_eq!(classify(&error, None, true), Failure::Protocol);
    }
}
