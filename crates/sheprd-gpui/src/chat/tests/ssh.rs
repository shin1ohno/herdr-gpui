use super::*;

#[test]
fn a_remote_chat_target_is_rejected_before_a_worker_or_process_starts() {
    let error = match super::super::link::Dial::ssh("-oProxyCommand=bad", "exec sheprd-acp agents")
    {
        Ok(_) => panic!("accepted an option as a host"),
        Err(error) => error,
    };
    assert!(matches!(
        error,
        Error::Remote(sheprd_client::Error::Ssh(
            sheprd_client::ssh::Error::InvalidTarget
        ))
    ));
}
