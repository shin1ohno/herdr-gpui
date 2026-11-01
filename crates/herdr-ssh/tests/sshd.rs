//! Explicitly opt-in: starts only the selected sshd in a private sandbox.
#![cfg(all(unix, not(target_os = "ios")))]
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::disallowed_methods)]

#[allow(dead_code)]
#[path = "../../test-support/sandbox.rs"]
mod sandbox;
#[path = "../../test-support/sshd.rs"]
mod sshd;

use anyhow::{Result, ensure};
use herdr_ssh::{ClientKey, Failure, KnownHosts, Pins, Scan, Target, TunnelExit};
use p256::ecdsa::{Signature, SigningKey, signature::Signer as _};
use sshd::Sshd;
use std::{
    io::{Read as _, Write as _},
    net::Shutdown,
    sync::atomic::AtomicBool,
    time::Duration,
};

const DEADLINE: Duration = Duration::from_secs(15);

fn software_key(seed: u8) -> ClientKey {
    let signing = SigningKey::from_slice(&[seed; 32]).unwrap();
    let public = signing
        .verifying_key()
        .to_sec1_point(false)
        .as_bytes()
        .to_vec();
    ClientKey::new(public, move |message| {
        let signature: Signature = signing.sign(message);
        Ok(signature.to_bytes().into())
    })
    .unwrap()
}

#[test]
#[ignore = "requires explicit HERDR_TEST_SSHD and HERDR_TEST_SSH_KEYGEN; starts a private sshd"]
fn private_sshd_scan_trust_exec_and_refused_key() -> Result<()> {
    let key = software_key(11);
    let server = Sshd::start(&key.authorized_keys_line("herdr-test"), "ed25519")?;
    let target = Target::parse(&server.target()?)?;
    let hosts = KnownHosts::new(server.directory().join("known_hosts"));
    let stop = AtomicBool::new(false);
    let Scan::New(keys) = herdr_ssh::scan(&target, &Pins::None, &stop)? else {
        anyhow::bail!("new private host must require trust");
    };
    ensure!(
        !keys.is_empty(),
        "scan must return a fingerprint to approve"
    );
    hosts.trust(&target, &keys)?;
    let pins = hosts.pins(&target)?;
    ensure!(herdr_ssh::scan(&target, &pins, &stop)? == Scan::Trusted);
    let (mut stream, tunnel) = herdr_ssh::exec(
        &target,
        &key,
        &pins,
        "/bin/cat; printf diagnostic >&2; exit 7",
    )?;
    stream.set_read_timeout(Some(DEADLINE))?;
    stream.set_write_timeout(Some(DEADLINE))?;
    let payload = b"herdr-private-ssh\n\0\xff";
    stream.write_all(payload)?;
    stream.shutdown(Shutdown::Write)?;
    let mut output = Vec::new();
    stream.read_to_end(&mut output)?;
    ensure!(output == payload);
    ensure!(
        tunnel.wait_exit(Duration::ZERO)
            == Some(TunnelExit::Remote {
                exit_status: Some(7),
                stderr_tail: b"diagnostic".to_vec()
            })
    );
    let (mut stream, tunnel) = herdr_ssh::exec(&target, &software_key(12), &pins, "true")?;
    stream.set_read_timeout(Some(DEADLINE))?;
    stream.read_to_end(&mut Vec::new())?;
    ensure!(tunnel.wait_exit(Duration::ZERO) == Some(TunnelExit::Ssh(Failure::Auth)));
    Ok(())
}

#[test]
#[ignore = "requires explicit HERDR_TEST_SSHD and HERDR_TEST_SSH_KEYGEN; starts a private sshd"]
fn private_sshd_rsa_host_uses_sha2() -> Result<()> {
    let key = software_key(13);
    let server = Sshd::start(&key.authorized_keys_line("herdr-test"), "rsa")?;
    let target = Target::parse(&server.target()?)?;
    let Scan::New(keys) = herdr_ssh::scan(&target, &Pins::None, &AtomicBool::new(false))? else {
        anyhow::bail!("new private RSA host must require trust");
    };
    ensure!(keys.iter().all(|key| key.algorithm() == "ssh-rsa"));
    let hosts = KnownHosts::new(server.directory().join("known_hosts"));
    hosts.trust(&target, &keys)?;
    // The client's allow-list has only rsa-sha2-512/256 for these pins.
    let (mut stream, tunnel) =
        herdr_ssh::exec(&target, &key, &hosts.pins(&target)?, "printf rsa-sha2")?;
    stream.set_read_timeout(Some(DEADLINE))?;
    let mut output = Vec::new();
    stream.read_to_end(&mut output)?;
    ensure!(output == b"rsa-sha2");
    ensure!(
        tunnel.wait_exit(Duration::ZERO)
            == Some(TunnelExit::Remote {
                exit_status: Some(0),
                stderr_tail: Vec::new()
            })
    );
    Ok(())
}
