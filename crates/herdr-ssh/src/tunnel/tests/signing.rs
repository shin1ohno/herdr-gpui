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
) -> Result<(Watched, Arc<SignGate>), Violation> {
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
        return Err(fixture("the signing closure never started"));
    }
    Ok((run, gate))
}

/// R2: no receipt while a signature runs; one soon after it returns.
fn r2_check(mutant: Option<Mutant>) -> Result<(), Violation> {
    let _turn = ONE_AT_A_TIME.lock().unwrap_or_else(PoisonError::into_inner);
    let host = TestHost::start(Behavior::Silent);
    let (run, gate) = gated_run(&host, 31, mutant)?;
    run.tunnel.stop();
    // Longer than the 1 s disconnect grace, after which only the signature
    // is left to wait for.
    let early = run.tunnel.wait_teardown(Duration::from_secs(3));
    gate.open();
    if early.is_some() {
        return Err(Violation::SettledWhileSigning);
    }
    let _ = receipt(&run.tunnel)?;
    if Seen::get(&host.seen.execs) != 0 {
        return Err(Violation::ExecAfterStop);
    }
    Ok(())
}

#[test]
fn r2_no_receipt_while_a_signature_runs() {
    holds(r2_check(None));
}

#[test]
fn r2_not_waiting_for_signers_is_caught() {
    assert_eq!(
        r2_check(Some(Mutant::IgnoreSigners)),
        Err(Violation::SettledWhileSigning)
    );
}

/// R3: a signature that never returns makes the teardown `Unproven`, about
/// six seconds after the stop (the 1 s grace, then the 5 s signer deadline).
fn r3_check(mutant: Option<Mutant>) -> Result<(), Violation> {
    let _turn = ONE_AT_A_TIME.lock().unwrap_or_else(PoisonError::into_inner);
    let host = TestHost::start(Behavior::Silent);
    let (run, gate) = gated_run(&host, 32, mutant)?;
    let stopped = Instant::now();
    run.tunnel.stop();
    let outcome = unproven(&run.tunnel);
    let elapsed = stopped.elapsed();
    gate.open();
    outcome?;
    if !(Duration::from_millis(4500)..=Duration::from_secs(9)).contains(&elapsed) {
        return Err(fixture(format!("settled {elapsed:?} after the stop")));
    }
    let alive = run
        .probe
        .alive_at_settle()
        .ok_or_else(|| fixture("no settle observed"))?;
    // The stuck closure keeps its own copy of the key, so `key` is 1 here;
    // the tunnel's own copy going on `Unproven` is R8's check.
    if alive.handler != 0 {
        return Err(Violation::HandlerHeld);
    }
    Ok(())
}

#[test]
fn r3_a_stuck_signature_is_unproven() {
    holds(r3_check(None));
}

#[test]
fn r3_completing_despite_a_stuck_signature_is_caught() {
    assert_eq!(
        r3_check(Some(Mutant::CompleteOnSignerTimeout)),
        Err(Violation::CompleteInsteadOfUnproven)
    );
}
