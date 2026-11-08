use super::*;
use control::{ControlCommand, Invocation};

fn parse(args: &[&str]) -> Result<Invocation, CliError> {
    LaunchOptions::parse(args.iter().copied()).map(|options| match options.mode {
        LaunchMode::Control(invocation) => invocation,
        mode => panic!("{args:?} parsed as {mode:?}"),
    })
}

#[test]
fn agent_pins_require_an_explicit_pane_and_honor_window_and_json() {
    for (verb, pinned) in [("pin", true), ("unpin", false)] {
        let invocation =
            parse(&["agent", verb, "--pane", "p0", "--json", "--window", "7"]).unwrap();
        assert!(invocation.json);
        assert_eq!(invocation.window, Some(7));
        assert_eq!(
            invocation.command,
            ControlCommand::Agent {
                pane: "p0".into(),
                pinned
            }
        );
    }
    for args in [
        &["agent"][..],
        &["agent", "pin"],
        &["agent", "remove", "--pane", "p0"],
        &["agent", "pin", "--pane", "p0", "extra"],
    ] {
        assert!(parse(args).is_err(), "{args:?}");
    }
}
