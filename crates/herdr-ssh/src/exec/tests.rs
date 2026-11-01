use super::*;

fn stderr(bytes: &[u8]) -> ChannelMsg {
    ChannelMsg::ExtendedData {
        data: bytes.to_vec().into(),
        ext: STDERR,
    }
}

fn tail(chunks: &[&[u8]]) -> Vec<u8> {
    let mut tail = Tail::default();
    for chunk in chunks {
        tail.push(chunk);
    }
    tail.into_vec()
}

#[test]
fn tail_keeps_the_last_bytes() {
    assert_eq!(tail(&[b"abc", b"def"]), b"abcdef");
    let big = vec![b'x'; STDERR_TAIL + 10];
    let kept = tail(&[b"head", &big, b"end"]);
    assert_eq!(kept.len(), STDERR_TAIL);
    assert!(kept.ends_with(b"xend"));
    let mut numbered: Vec<u8> = (0..=255u8).cycle().take(STDERR_TAIL * 2 + 7).collect();
    let expected = numbered.split_off(numbered.len() - STDERR_TAIL);
    let all: Vec<u8> = [numbered.as_slice(), &expected].concat();
    assert_eq!(tail(&[&all]), expected);
    let pieces: Vec<&[u8]> = all.chunks(333).collect();
    assert_eq!(tail(&pieces), expected);
}

#[test]
fn exit_status_and_stderr_end_the_command() {
    let mut output = Output::default();
    assert_eq!(output.receive(ChannelMsg::Success), Step::Continue);
    assert_eq!(output.receive(stderr(b"warning\n")), Step::Continue);
    // Another extended stream is not standard error.
    assert_eq!(
        output.receive(ChannelMsg::ExtendedData {
            data: b"other".to_vec().into(),
            ext: 2,
        }),
        Step::Continue
    );
    assert_eq!(output.receive(ChannelMsg::Eof), Step::Eof);
    assert_eq!(
        output.receive(ChannelMsg::ExitStatus { exit_status: 3 }),
        Step::Continue
    );
    assert_eq!(output.receive(ChannelMsg::Close), Step::Closed);
    assert_eq!(
        output.finish(),
        Some(TunnelExit::Remote {
            exit_status: Some(3),
            stderr_tail: b"warning\n".to_vec(),
        })
    );
}

#[test]
fn remote_text_never_becomes_an_ssh_failure() {
    // B3: what the command writes is only its stderr tail.
    let mut output = Output::default();
    output.receive(ChannelMsg::Success);
    output.receive(stderr(
        b"Host key verification failed.\r\nPermission denied (publickey).\r\n",
    ));
    output.receive(ChannelMsg::ExitStatus { exit_status: 255 });
    output.receive(ChannelMsg::Close);
    assert!(matches!(
        output.finish(),
        Some(TunnelExit::Remote {
            exit_status: Some(255),
            ..
        })
    ));
}

#[test]
fn refused_exec_is_a_protocol_failure() {
    let mut output = Output::default();
    assert_eq!(output.receive(ChannelMsg::Failure), Step::Refused);
    // After acceptance a failure answers something else and is ignored.
    let mut output = Output::default();
    output.receive(ChannelMsg::Success);
    assert_eq!(output.receive(ChannelMsg::Failure), Step::Continue);
}

#[test]
fn signal_has_no_exit_status() {
    let mut output = Output::default();
    output.receive(ChannelMsg::ExitSignal {
        signal_name: russh::Sig::KILL,
        core_dumped: false,
        error_message: "server text".into(),
        lang_tag: String::new(),
    });
    assert_eq!(
        output.finish(),
        Some(TunnelExit::Remote {
            exit_status: None,
            stderr_tail: Vec::new(),
        })
    );
}

#[test]
fn session_lost_before_the_command_ended_is_not_remote() {
    let mut output = Output::default();
    output.receive(ChannelMsg::Success);
    output.receive(stderr(b"partial"));
    assert_eq!(output.finish(), None);
}

mod eof;
