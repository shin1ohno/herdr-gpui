use super::*;

#[test]
fn a_remote_chat_target_is_rejected_before_a_worker_or_process_starts() {
    let error = match super::super::link::Dial::ssh("-oProxyCommand=bad", "exec herdr-acp agents") {
        Ok(_) => panic!("accepted an option as a host"),
        Err(error) => error,
    };
    assert!(matches!(
        error,
        Error::Remote(herdr_client::Error::Ssh(
            herdr_client::ssh::Error::InvalidTarget
        ))
    ));
}
