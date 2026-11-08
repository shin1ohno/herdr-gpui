//! The bridge on iPadOS, which cannot start `ssh` (docs/ios.md, "SSH
//! transport"). The same bridge command runs on the host over the in-process
//! client's exec channel, whose standard streams are one end of a socket
//! pair, so the ready handshake and the health pings are the desktop's.
//!
//! Only a host with a pinned key is connected to: an unpinned one fails as
//! a host key the user must approve, before any packet is sent, and a
//! reconnect never asks. Failures are classified from the tunnel's typed
//! exit (see `ipad`), never from what the host wrote.
use super::{Bridge, Destination, HostKeys, HostProbe, SshChannel, SshFailure, ipad, mux};
use crate::{Result, catalog::validate_target, limits::POLL, session_socket, transport::Stream};
use herdr_ssh::{ClientKey, KnownHosts, Pins, SignError, Target, Tunnel};
use sheprd_credentials::device_key::DeviceKey;
use std::{
    net::Shutdown,
    path::{Path, PathBuf},
    process::{Command, ExitStatus},
    sync::atomic::{AtomicBool, Ordering},
    time::{Duration, Instant},
};

pub(super) mod error;
pub use error::Error;

/// The Keychain label of this installation's SSH key.
const KEY_LABEL: &str = "herdr-ssh";
/// How long a closed tunnel may take to report how it ended, as the desktop
/// waits for a closed `ssh` to exit.
const DIAGNOSE_TIMEOUT: Duration = Duration::from_secs(2);

/// The bridge to `session` on `target`, with the tunnel that keeps it open.
pub(crate) fn connect(target: &str, session: &str, stop: &AtomicBool) -> Result<(Stream, Tunnel)> {
    validate_target(target)?;
    session_socket(Path::new(""), session)?;
    start_bridge(target, &super::bridge_command(session), stop)
}

/// A caller-owned binary protocol over an SSH exec channel, using the same
/// device key and pinned-host policy as the Herdr 1 bridge. The caller owns
/// command construction, framing and deadlines and keeps the tunnel alive.
/// Keychain and known-hosts reads block: call only from a background worker.
pub fn exec_ssh_stream(target: &str, remote_command: &str) -> Result<(Stream, Tunnel)> {
    validate_target(target)?;
    spawn(target, remote_command)
}

/// [`super::probe_host`] over the in-process client. Every host key is
/// verified on iPadOS, so `keys` changes nothing; a host with no pinned key
/// fails as [`SshFailure::HostKey`] without being connected to. Blocks for
/// at most `super::PROBE_TIMEOUT`: call it from a background thread.
pub fn probe_host_with(target: &str, session: &str, _keys: HostKeys) -> Result<HostProbe> {
    validate_target(target)?;
    session_socket(Path::new(""), session)?;
    tracing::info!(category = "probe", "probing host");
    probe(target, &super::probe_command(session))
}

/// `target` as the in-process client connects to it. iPadOS has no SSH
/// configuration, so this only parses and normalizes `user@host[:port]`.
pub fn resolve_destination(target: &str) -> Result<Destination> {
    validate_target(target)?;
    let Target { user, host, port } = parse(target)?;
    Ok(Destination { user, host, port })
}

/// iPadOS cannot start the `ssh` child a port forward is.
pub fn forward_command(
    target: &str,
    _local: u16,
    _address: Option<std::net::IpAddr>,
    _remote: u16,
) -> Result<Command> {
    validate_target(target)?;
    Err(super::Error::Unsupported.into())
}

/// iPadOS cannot start the `ssh` child a host script runs in.
pub fn script_command(target: &str, _script: &str) -> Result<Command> {
    validate_target(target)?;
    Err(Error::ProcessUnsupported.into())
}

/// iPadOS cannot start the `ssh` child a host script runs in, and keeps no
/// shared connections.
pub fn shared_script_command(
    target: &str,
    _script: &str,
    _lane: mux::Lane,
) -> Result<(Command, Option<SshChannel>)> {
    validate_target(target)?;
    Err(Error::ProcessUnsupported.into())
}

/// iPadOS cannot start an application-built command either.
pub fn connect_command(_command: Command, session: &str, _stop: &AtomicBool) -> Result<Bridge> {
    session_socket(Path::new(""), session)?;
    Err(Error::ProcessUnsupported.into())
}

/// Runs a bounded command through the same pinned connection as the bridge.
/// No PTY, forwarding, environment, or local process is involved. A closed
/// stream is successful only after the remote command reports its exit.
pub fn run_remote(
    target: &str,
    remote_command: &str,
    timeout: Duration,
    cancelled: impl Fn() -> bool,
) -> Result<(ExitStatus, Vec<u8>)> {
    run_ssh_command(
        target,
        remote_command,
        timeout,
        super::PROBE_OUTPUT_LIMIT,
        cancelled,
    )
}

/// Runs an application command with its own stdout cap, never exceeding the
/// client's hard probe limit. Blocks: call only from a background worker.
pub fn run_ssh_command(
    target: &str,
    remote_command: &str,
    timeout: Duration,
    output_limit: u64,
    cancelled: impl Fn() -> bool,
) -> Result<(ExitStatus, Vec<u8>)> {
    let deadline = Instant::now() + timeout;
    if cancelled() {
        return Err(super::Error::Cancelled.into());
    }
    let (mut stream, tunnel) = spawn(target, remote_command)?;
    stream.shutdown(Shutdown::Write)?;
    stream.set_read_timeout(Some(POLL))?;
    let output = super::exec_output::read(&mut stream, deadline, output_limit, &cancelled)?;
    loop {
        if cancelled() {
            return Err(super::Error::Cancelled.into());
        }
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return Err(super::Error::Timeout.into());
        }
        let Some(exit) = tunnel.wait_exit(remaining.min(DIAGNOSE_SLICE)) else {
            continue;
        };
        return match exit {
            herdr_ssh::TunnelExit::Remote { exit_status, .. } => {
                Ok((ipad::remote_status(exit_status), output))
            }
            herdr_ssh::TunnelExit::Ssh(herdr_ssh::Failure::Cancelled) => {
                Err(super::Error::Cancelled.into())
            }
            herdr_ssh::TunnelExit::Ssh(failure) => {
                Err(super::Error::Refused(ipad::ssh_failure(failure)).into())
            }
        };
    }
}

/// Starts `remote_command` on `target` and returns this end of its standard
/// streams at once; the connection proceeds on the client's runtime, within
/// the bridge's ready deadline. Blocks on the Keychain and the known hosts
/// file: call it off the UI thread.
fn spawn(target: &str, remote_command: &str) -> Result<(Stream, Tunnel)> {
    let target = parse(target)?;
    let pins = known_hosts()?.pins(&target).map_err(Error::Ssh)?;
    if pins == Pins::None {
        tracing::warn!(category = "ssh", "host has no trusted key");
        return Err(super::Error::Refused(SshFailure::HostKey).into());
    }
    let key = client_key()?;
    Ok(herdr_ssh::exec(&target, &key, &pins, remote_command).map_err(Error::Ssh)?)
}

/// The bridge, ready for the client protocol once the remote accepted the
/// handshake. A tunnel that closes first is classified from how it ended.
fn start_bridge(target: &str, remote_command: &str, stop: &AtomicBool) -> Result<(Stream, Tunnel)> {
    let (mut stream, tunnel) = spawn(target, remote_command)?;
    stream.set_read_timeout(Some(POLL))?;
    stream.set_write_timeout(Some(Duration::from_secs(1)))?;
    match super::handshake(&mut stream, stop) {
        Err(crate::Error::Ssh(super::Error::Closed)) => {
            let failure = diagnose(&tunnel, stop);
            tracing::warn!(
                category = "ssh",
                ?failure,
                "bridge closed before it was ready"
            );
            Err(super::Error::Refused(failure).into())
        }
        result => result.map(|()| (stream, tunnel)),
    }
}

/// How a tunnel whose stream closed ended, waiting at most
/// `DIAGNOSE_TIMEOUT`. A tunnel that does not say in time, or a stop, reads
/// as `Other`, as a desktop `ssh` that does not exit in time does.
fn diagnose(tunnel: &Tunnel, stop: &AtomicBool) -> SshFailure {
    let deadline = Instant::now() + DIAGNOSE_TIMEOUT;
    loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if let Some(exit) = tunnel.wait_exit(remaining.min(DIAGNOSE_SLICE)) {
            return ipad::exit_failure(&exit);
        }
        if remaining.is_zero() || stop.load(Ordering::Acquire) {
            return SshFailure::Other;
        }
    }
}

/// How long one wait for a tunnel's exit lasts before the stop flag is read.
const DIAGNOSE_SLICE: Duration = Duration::from_millis(100);

/// Runs the probe script and classifies what it printed, as the desktop
/// does with an `ssh` child.
fn probe(target: &str, remote_command: &str) -> Result<HostProbe> {
    let (status, output) = match run_remote(target, remote_command, super::PROBE_TIMEOUT, || false)
    {
        Ok(result) => result,
        Err(crate::Error::Ssh(super::Error::Refused(failure))) => {
            return Ok(HostProbe::SshFailed(failure));
        }
        Err(error) => return Err(error),
    };
    if !status.success() {
        return Err(crate::Error::RemoteCommand(status));
    }
    let probe = super::classify_probe(&output);
    tracing::info!(
        category = "probe",
        stdout_bytes = output.len(),
        ?probe,
        "probe finished"
    );
    let probe = probe.ok_or(super::Error::Closed)?;
    Ok(ipad::with_core_probe(probe, || {
        let (status, output) = run_remote(
            target,
            &super::core::probe_command(),
            super::PROBE_TIMEOUT,
            || false,
        )?;
        Ok(status.success() && output == b"herdr-core-available:1\n")
    }))
}

/// `target` as the in-process client reads it: `user@host[:port]`, the user
/// required, with nothing from an SSH config.
pub(super) fn parse(target: &str) -> Result<Target> {
    Ok(Target::parse(target).map_err(Error::Ssh)?)
}

/// Where the app pins host keys: its own `known_hosts`, kept with its other
/// state and apart from any user file.
pub(super) fn known_hosts_path() -> Result<PathBuf> {
    let dir = crate::local_path::state_dir().ok_or(Error::StateDir)?;
    Ok(dir.join("ssh").join("known_hosts"))
}

pub(super) fn known_hosts() -> Result<KnownHosts> {
    Ok(KnownHosts::new(known_hosts_path()?))
}

/// This installation's key, created on first use. The private half stays
/// with the device key; only its signatures reach the client.
fn client_key() -> Result<ClientKey> {
    let device = DeviceKey::load_or_create(KEY_LABEL).map_err(Error::DeviceKey)?;
    client_key_from(device)
}

fn client_key_from(device: DeviceKey) -> Result<ClientKey> {
    let public = device.public_sec1();
    let sign = move |message: &[u8]| {
        device.sign(message).map_err(|error| {
            tracing::warn!(category = "ssh", %error, "device key did not sign");
            SignError {
                reason: "the device key refused to sign",
            }
        })
    };
    Ok(ClientKey::new(public, sign).map_err(Error::Ssh)?)
}

/// The line that lets this iPad's key in, for the user to add to
/// `~/.ssh/authorized_keys` on a computer: restricted to running commands,
/// with no PTY or forwarding, and named after this device. Creates the key
/// on first use; blocks on the Keychain, so call it off the UI thread.
pub fn authorized_key() -> Result<String> {
    Ok(authorized_key_line(&client_key()?))
}

/// Replaces this installation's device key and returns the new restricted
/// `authorized_keys` line. Each new connection reads the Keychain again, so
/// it uses this new key automatically. Live tunnels keep their authenticated
/// sessions; replacement neither disconnects nor reconnects them. Every
/// computer needs the new line installed and the old line removed.
/// Blocks on the Keychain: call only after user confirmation, off the UI thread.
pub fn replace_authorized_key() -> Result<String> {
    let device = DeviceKey::replace(KEY_LABEL).map_err(Error::DeviceKey)?;
    Ok(authorized_key_line(&client_key_from(device)?))
}

fn authorized_key_line(key: &ClientKey) -> String {
    let device = rustix::system::uname();
    let name = device.nodename().to_string_lossy();
    key.authorized_keys_line(&ipad::key_comment(&name))
}
