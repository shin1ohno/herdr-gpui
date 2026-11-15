//! R2 and R3: a signing closure that is still running holds the receipt
//! back, and one that does not return within its deadline makes the
//! teardown `Unproven`. The stop lands while authentication waits on the
//! closure, inside the setup stage, so the timeline also shows that a stop
//! there reaches the teardown at once (rather than after the setup
//! deadline).

use super::*;
use crate::test_host::{Behavior, Seen, TestHost};

/// The runtime has two blocking threads; these tests each hold one, so they
/// take turns rather than starve each other's signatures.
static ONE_AT_A_TIME: Mutex<()> = Mutex::new(());

fn gated_run(
    host: &TestHost,
    seed: u8,
    mutant: Option<Mutant>,
) -> Result<(Watched, Arc<SignGate>), String> {
    let gate = Arc::new(SignGate::default());
    let run = start_watched(
        &target(host.port),
        &host.pins,
        watched_key(seed, Some(Arc::clone(&gate))),
        "signed",
        Deadlines::default(),
        removing(mutant),
    );
    if !gate.wait_entered(SETTLE) {
        gate.open();
        return Err("the signing closure never started".into());
    }
    Ok((run, gate))
}

/// R2: no receipt while a signature runs; one soon after it returns.
fn r2_check(mutant: Option<Mutant>) -> Result<(), String> {
    let _turn = ONE_AT_A_TIME.lock().unwrap_or_else(PoisonError::into_inner);
    let host = TestHost::start(Behavior::Silent);
    let (run, gate) = gated_run(&host, 31, mutant)?;
    run.tunnel.stop();
    // Longer than the 1 s disconnect grace, after which only the signature
    // is left to wait for.
    let early = run.tunnel.wait_teardown(Duration::from_secs(3));
    gate.open();
    if early.is_some() {
        return Err(format!("settled while signing: {early:?}"));
    }
    let _ = receipt(&run.tunnel)?;
    if Seen::get(&host.seen.execs) != 0 {
        return Err("an exec request was sent after the stop".into());
    }
    Ok(())
}

#[test]
fn r2_no_receipt_while_a_signature_runs() {
    r2_check(None).unwrap();
}

#[test]
fn r2_not_waiting_for_signers_is_caught() {
    assert!(r2_check(Some(Mutant::IgnoreSigners)).is_err());
}

/// R3: a signature that never returns makes the teardown `Unproven`, about
/// five seconds after the stop.
fn r3_check(mutant: Option<Mutant>) -> Result<(), String> {
    let _turn = ONE_AT_A_TIME.lock().unwrap_or_else(PoisonError::into_inner);
    let host = TestHost::start(Behavior::Silent);
    let (run, gate) = gated_run(&host, 32, mutant)?;
    let stopped = Instant::now();
    run.tunnel.stop();
    let outcome = run.tunnel.wait_teardown(SETTLE);
    let elapsed = stopped.elapsed();
    gate.open();
    match outcome {
        Some(Teardown::Unproven) => {}
        other => return Err(format!("expected unproven, got {other:?}")),
    }
    if elapsed < Duration::from_millis(4500) || elapsed > Duration::from_secs(9) {
        return Err(format!("settled {elapsed:?} after the stop"));
    }
    let alive = run.probe.alive_at_settle().ok_or("no settle observed")?;
    // The stuck closure keeps its own copy of the key; the tunnel kept none.
    if alive.key != 1 || alive.handler != 0 {
        return Err(format!("unexpected holders at the settle: {alive:?}"));
    }
    Ok(())
}

#[test]
fn r3_a_stuck_signature_is_unproven() {
    r3_check(None).unwrap();
}

#[test]
fn r3_completing_despite_a_stuck_signature_is_caught() {
    assert!(r3_check(Some(Mutant::CompleteOnSignerTimeout)).is_err());
}
