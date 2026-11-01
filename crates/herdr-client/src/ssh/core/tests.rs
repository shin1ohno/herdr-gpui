#![allow(clippy::unwrap_used)]
use super::*;

#[test]
fn core_session_is_validated_before_constructing_a_remote_command() {
    for session in ["", ".", "..", "../outside", "$(touch marker)", "x\n"] {
        assert!(matches!(
            core_connect_command(session),
            Err(crate::Error::InvalidSession)
        ));
    }
    let command = core_connect_command("agents").unwrap();
    assert!(command.starts_with("/bin/sh -c '"));
    assert!(command.contains("connect --session"));
    assert!(command.contains("command -v hc"));
    assert!(command.contains("command -v herdr-core"));
    assert!(command.ends_with("exit 127'"));
    assert!(!command.contains("server start"));
    assert!(!command.contains("server stop"));
}
