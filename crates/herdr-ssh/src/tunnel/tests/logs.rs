//! R11: the tunnel's logs, the teardown's included, carry no private value.
//! A process-wide subscriber captures every event (logs run on the
//! runtime's thread, so a thread-local one would see nothing); each run
//! uses its own user name and command, so other tests' events cannot hide
//! or fake a leak.

use std::{
    fmt::{self, Write as _},
    sync::Once,
};

use tracing::{
    Metadata, Subscriber,
    field::{Field, Visit},
    span,
};

use super::*;
use crate::test_host::{Behavior, TestHost};

static CAPTURED: Mutex<Vec<String>> = Mutex::new(Vec::new());

struct Capture;

struct Line<'a>(&'a mut String);

impl Visit for Line<'_> {
    fn record_debug(&mut self, field: &Field, value: &dyn fmt::Debug) {
        let _ = write!(self.0, "{}={value:?} ", field.name());
    }

    fn record_str(&mut self, field: &Field, value: &str) {
        let _ = write!(self.0, "{}={value} ", field.name());
    }
}

impl Subscriber for Capture {
    fn enabled(&self, _metadata: &Metadata<'_>) -> bool {
        true
    }

    fn new_span(&self, _span: &span::Attributes<'_>) -> span::Id {
        span::Id::from_u64(1)
    }

    fn record(&self, _span: &span::Id, _values: &span::Record<'_>) {}

    fn record_follows_from(&self, _span: &span::Id, _follows: &span::Id) {}

    fn event(&self, event: &tracing::Event<'_>) {
        let mut line = String::new();
        event.record(&mut Line(&mut line));
        CAPTURED.lock().unwrap().push(line);
    }

    fn enter(&self, _span: &span::Id) {}

    fn exit(&self, _span: &span::Id) {}
}

fn captured() -> Vec<String> {
    static INSTALL: Once = Once::new();
    INSTALL.call_once(|| {
        tracing::subscriber::set_global_default(Capture).unwrap();
    });
    CAPTURED.lock().unwrap().clone()
}

fn teardown_lines(lines: &[String]) -> usize {
    lines
        .iter()
        .filter(|line| line.contains("ssh tunnel teardown"))
        .count()
}

fn r11_check(mutant: Option<Mutant>, user: &str) -> Result<(), String> {
    let before = teardown_lines(&captured());
    let host = TestHost::start(Behavior::Exit {
        stdout: b"ok".to_vec(),
        status: 0,
    });
    let command = format!("command-of-{user}");
    let key = watched_key(60, None);
    let key_line = key.0.authorized_keys_line("");
    let key_text = key_line.split(' ').nth(2).ok_or("no key field")?.to_owned();
    let Pins::Keys(pins) = &host.pins else {
        return Err("the host has no pins".into());
    };
    let pin_texts: Vec<String> = pins.iter().map(HostKey::fingerprint).collect();
    let address = format!("{user}@127.0.0.1:{}", host.port);
    let target = Target::parse(&address).map_err(|e| e.to_string())?;
    let mut run = start_watched(
        &target,
        &host.pins,
        key,
        &command,
        Deadlines::default(),
        removing(mutant),
    );
    run.stream
        .read_to_end(&mut Vec::new())
        .map_err(|e| e.to_string())?;
    let _ = receipt(&run.tunnel)?;
    // And a failing tunnel with the same user.
    let failing =
        Target::parse(&format!("{user}@127.0.0.1:{}", closed_port())).map_err(|e| e.to_string())?;
    let failed = start_watched(
        &failing,
        &host.pins,
        watched_key(61, None),
        &command,
        Deadlines::default(),
        removing(mutant),
    );
    let _ = receipt(&failed.tunnel)?;
    let lines = captured();
    // Positive control: the capture saw this test's teardowns.
    if teardown_lines(&lines) < before + 2 {
        return Err("the teardown events were not captured".into());
    }
    let private = [user, command.as_str(), key_text.as_str(), address.as_str()];
    for line in &lines {
        for value in private
            .iter()
            .copied()
            .chain(pin_texts.iter().map(String::as_str))
        {
            if line.contains(value) {
                return Err(format!("a log line carries a private value: {line}"));
            }
        }
    }
    Ok(())
}

#[test]
fn r11_logs_carry_no_private_value() {
    r11_check(None, "r11-sentinel-plain").unwrap();
}

#[test]
fn r11_logging_the_target_is_caught() {
    assert!(r11_check(Some(Mutant::LogTarget), "r11-sentinel-mutant").is_err());
}
