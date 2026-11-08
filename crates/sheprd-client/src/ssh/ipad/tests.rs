use super::*;

mod probe;

const EVERY_FAILURE: &[Failure] = &[
    Failure::HostKeyUnknown,
    Failure::HostKeyChanged,
    Failure::Auth,
    Failure::Unreachable,
    Failure::Timeout,
    Failure::Protocol,
    Failure::Cancelled,
];

#[test]
fn a_remote_exit_status_cannot_wrap_to_success() {
    assert!(remote_status(Some(0)).success());
    assert_eq!(remote_status(Some(127)).code(), Some(127));
    assert_eq!(remote_status(Some(255)).code(), Some(255));
    for code in [256, 65536, u32::MAX] {
        assert!(!remote_status(Some(code)).success());
        assert_eq!(remote_status(Some(code)).code(), Some(255));
    }
    assert!(!remote_status(None).success());
    assert_eq!(remote_status(None).code(), None);
}

#[test]
fn an_unknown_or_changed_host_key_needs_the_user() {
    for failure in [Failure::HostKeyUnknown, Failure::HostKeyChanged] {
        let class = ssh_failure(failure);
        assert_eq!(class, SshFailure::HostKey, "{failure:?}");
        // The endpoint backs off to its longest delay and never prompts.
        assert!(class.needs_user(), "{failure:?}");
        assert_eq!(exit_failure(&TunnelExit::Ssh(failure)), SshFailure::HostKey);
    }
}

#[test]
fn ssh_layer_failures_map_to_their_desktop_classes() {
    assert_eq!(ssh_failure(Failure::Auth), SshFailure::Auth);
    assert!(ssh_failure(Failure::Auth).needs_user());
    assert_eq!(ssh_failure(Failure::Unreachable), SshFailure::Unreachable);
    assert_eq!(ssh_failure(Failure::Timeout), SshFailure::Unreachable);
    assert_eq!(ssh_failure(Failure::Protocol), SshFailure::Other);
    assert_eq!(ssh_failure(Failure::Cancelled), SshFailure::Other);
    // Only the host key and authentication stop the reconnect loop.
    for &failure in EVERY_FAILURE {
        let needs_user = matches!(
            failure,
            Failure::HostKeyUnknown | Failure::HostKeyChanged | Failure::Auth
        );
        assert_eq!(ssh_failure(failure).needs_user(), needs_user, "{failure:?}");
    }
}

#[test]
fn remote_stderr_never_names_an_ssh_class() {
    // What a desktop `ssh` prints for these classes, written by the remote
    // program instead and exiting as `ssh` itself would.
    for stderr in [
        &b"Host key verification failed.\n"[..],
        b"@@@ WARNING: REMOTE HOST IDENTIFICATION HAS CHANGED! @@@\n",
        b"Permission denied (publickey).\n",
        b"Too many authentication failures\n",
        b"ssh: connect to host 10.0.0.2 port 22: No route to host\n",
    ] {
        for exit_status in [Some(255), Some(1), None] {
            let exit = TunnelExit::Remote {
                exit_status,
                stderr_tail: stderr.to_vec(),
            };
            assert_eq!(exit_failure(&exit), SshFailure::Other, "{exit:?}");
        }
    }
}

#[test]
fn a_bridge_script_without_herdr_exits_127() {
    let exit = TunnelExit::Remote {
        exit_status: Some(127),
        stderr_tail: b"Host key verification failed.\n".to_vec(),
    };
    assert_eq!(exit_failure(&exit), SshFailure::HerdrMissing);
    let exit = TunnelExit::Remote {
        exit_status: Some(0),
        stderr_tail: Vec::new(),
    };
    assert_eq!(exit_failure(&exit), SshFailure::Other);
}

#[test]
fn host_key_algorithms_read_as_ssh_keygen_types() {
    assert_eq!(key_kind("ssh-ed25519"), Some(("ED25519", 256)));
    assert_eq!(key_kind("ecdsa-sha2-nistp256"), Some(("ECDSA", 256)));
    assert_eq!(key_kind("ecdsa-sha2-nistp384"), Some(("ECDSA", 384)));
    assert_eq!(key_kind("ecdsa-sha2-nistp521"), Some(("ECDSA", 521)));
    for rsa in ["ssh-rsa", "rsa-sha2-256", "rsa-sha2-512"] {
        assert_eq!(key_kind(rsa), Some(("RSA", 0)), "{rsa}");
    }
    assert_eq!(key_kind("ssh-dss"), None);
    assert_eq!(key_kind(""), None);
}

#[test]
fn the_key_comment_is_one_safe_word() {
    assert_eq!(key_comment("Shins-iPad"), "herdr-ipad-Shins-iPad");
    assert_eq!(key_comment("Shins-iPad.local"), "herdr-ipad-Shins-iPad");
    assert_eq!(key_comment("Shin's iPad Pro"), "herdr-ipad-Shin-s-iPad-Pro");
    assert_eq!(key_comment("  iPad  "), "herdr-ipad-iPad");
    assert_eq!(key_comment("イパッド"), "herdr-ipad");
    assert_eq!(key_comment(""), "herdr-ipad");
    assert_eq!(key_comment("a\nb\tc"), "herdr-ipad-a-b-c");
    let long = key_comment(&"x".repeat(200));
    assert_eq!(long, format!("herdr-ipad-{}", "x".repeat(NAME_LIMIT)));
    for comment in [key_comment("a b"), key_comment("\u{0}")] {
        assert!(
            comment
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '.' | '_')),
            "{comment}"
        );
    }
}
