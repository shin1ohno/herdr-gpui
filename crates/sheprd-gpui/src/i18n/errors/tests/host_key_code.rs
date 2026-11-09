use super::*;
use crate::host_key_code::Error;
use std::{error::Error as _, io, path::PathBuf};

#[test]
fn every_host_key_code_error_is_localized_without_an_english_leak() {
    let path = PathBuf::from("/test/ssh_host_ed25519_key.pub");
    let qr = sheprd_remote_control::app::qr::Qr::encode(&"x".repeat(5000))
        .err()
        .unwrap();
    assert_localized(&[
        Error::Fingerprints,
        Error::HostHint,
        Error::Invalid,
        Error::NoHostKeys,
        Error::PublicKey { path: path.clone() },
        Error::Qr(qr),
        Error::Read {
            path,
            source: io::Error::new(io::ErrorKind::PermissionDenied, "permission denied"),
        },
        Error::TooLarge,
        Error::Unsupported,
        Error::Version,
    ]);
}

#[test]
fn host_key_code_read_failure_keeps_its_original_io_source() {
    let error = Error::Read {
        path: PathBuf::from("/test/ssh_host_rsa_key.pub"),
        source: io::Error::new(io::ErrorKind::PermissionDenied, "permission denied"),
    };
    let source = error.source().unwrap().downcast_ref::<io::Error>().unwrap();
    assert_eq!(source.kind(), io::ErrorKind::PermissionDenied);
    let error = crate::Error::from(error);
    assert!(matches!(
        error,
        crate::Error::HostKeyCode(Error::Read { .. })
    ));
    assert_eq!(
        error
            .source()
            .unwrap()
            .downcast_ref::<io::Error>()
            .unwrap()
            .kind(),
        io::ErrorKind::PermissionDenied
    );
}
