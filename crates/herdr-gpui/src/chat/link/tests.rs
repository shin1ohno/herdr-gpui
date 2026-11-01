use super::*;

#[test]
fn ssh_failures_keep_the_typed_cause_while_remote_stderr_never_classifies_them() {
    for failure in [
        herdr_ssh::Failure::HostKeyChanged,
        herdr_ssh::Failure::HostKeyUnknown,
        herdr_ssh::Failure::Auth,
        herdr_ssh::Failure::Unreachable,
    ] {
        assert!(matches!(
            ssh_exit_failure(Some(herdr_ssh::TunnelExit::Ssh(failure))),
            Some(Error::Ssh(actual)) if actual == failure
        ));
    }
    assert!(ssh_exit_failure(None).is_none());
    assert!(
        ssh_exit_failure(Some(herdr_ssh::TunnelExit::Remote {
            exit_status: Some(255),
            stderr_tail: b"WARNING: REMOTE HOST IDENTIFICATION HAS CHANGED!".to_vec(),
        }))
        .is_none()
    );
}
