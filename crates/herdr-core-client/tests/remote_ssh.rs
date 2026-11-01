//! Explicitly selected binaries, a private sshd and core, and no user daemon.
#![cfg(all(unix, not(target_os = "ios")))]
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::disallowed_methods)]

#[allow(dead_code)]
#[path = "../../test-support/sandbox.rs"]
mod sandbox;
#[path = "../../test-support/sshd.rs"]
mod sshd;

use anyhow::{Result, ensure};
use herdr_core_client::{
    Core, Endpoint, Error,
    wire::{Attach, Run},
};
use herdr_ssh::{ClientKey, KnownHosts, Pins, Scan, Target, Tunnel};
use p256::ecdsa::{Signature, SigningKey, signature::Signer as _};
use sandbox::Sandbox;
use std::{
    ffi::OsString,
    io::{Read, Write},
    os::unix::net::UnixStream,
    path::PathBuf,
    process::{Child, Command},
    sync::atomic::AtomicBool,
    thread,
    time::{Duration, Instant},
};

const WAIT: Duration = Duration::from_secs(15);
const SESSION: &str = "ssh-live";

struct PrivateCore {
    child: Option<Child>,
    sandbox: Sandbox,
    binary: PathBuf,
    endpoint: Endpoint,
}

impl PrivateCore {
    fn start() -> Result<Self> {
        let binary = sshd::executable("HERDR_CORE_TEST_BINARY")?;
        let sandbox = Sandbox::new();
        let endpoint = Endpoint {
            socket: sandbox.dir.join("core.sock"),
            session: SESSION.into(),
        };
        let mut core = Self {
            child: None,
            sandbox,
            binary,
            endpoint,
        };
        core.child = Some(core.command("core.log").arg("serve").spawn()?);
        let deadline = Instant::now() + WAIT;
        loop {
            match Core::connect(&core.endpoint) {
                Ok(probe) => {
                    drop(probe);
                    break;
                }
                Err(Error::Connect { .. }) if Instant::now() < deadline => {
                    ensure!(
                        core.child.as_mut().unwrap().try_wait()?.is_none(),
                        "private core exited during startup"
                    );
                    thread::sleep(Duration::from_millis(20));
                }
                Err(error) => return Err(error.into()),
            }
        }
        Ok(core)
    }

    fn command(&self, log: &str) -> Command {
        let mut command = self.sandbox.command(&self.binary, log);
        command
            .env("HC_SESSION", SESSION)
            .env("HC_SOCKET", &self.endpoint.socket);
        command
    }

    fn ssh_command(&self) -> Result<String> {
        let path = std::env::join_paths([
            self.binary.parent().unwrap(),
            std::path::Path::new("/usr/bin"),
            std::path::Path::new("/bin"),
            std::path::Path::new("/usr/sbin"),
            std::path::Path::new("/sbin"),
        ])?;
        // sshd sets HOME for the login account. Reset all core discovery inputs
        // inside the remote command as well, so hc can only reach this fixture.
        let values: [(&str, OsString); 9] = [
            ("HOME", self.sandbox.dir.clone().into_os_string()),
            ("PATH", path),
            ("HC_SESSION", SESSION.into()),
            ("HC_SOCKET", self.endpoint.socket.clone().into_os_string()),
            (
                "XDG_CONFIG_HOME",
                self.sandbox.dir.join("config").into_os_string(),
            ),
            (
                "XDG_STATE_HOME",
                self.sandbox.dir.join("state").into_os_string(),
            ),
            ("XDG_RUNTIME_DIR", self.sandbox.dir.clone().into_os_string()),
            ("SHELL", "/bin/sh".into()),
            ("TERM", "xterm-256color".into()),
        ];
        let mut command = "/usr/bin/env -i".to_owned();
        for (name, value) in values {
            command.push(' ');
            command.push_str(name);
            command.push('=');
            command.push_str(&quote(
                value
                    .to_str()
                    .ok_or_else(|| anyhow::anyhow!("test path must be UTF-8"))?,
            ));
        }
        command.push(' ');
        command.push_str(&herdr_client::core_connect_command(SESSION)?);
        Ok(command)
    }
}

fn reap(child: &mut Option<Child>) {
    let Some(child) = child.as_mut() else {
        return;
    };
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        if child.try_wait().ok().flatten().is_some() {
            break;
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            break;
        }
        thread::sleep(Duration::from_millis(20));
    }
}

impl Drop for PrivateCore {
    fn drop(&mut self) {
        // This core alone owns holder processes, so use the allowed stop through
        // its exact private HC_SOCKET before reaping the serve child we created.
        let mut stopping = self
            .command("core-stop.log")
            .args(["server", "stop"])
            .spawn()
            .ok();
        reap(&mut stopping);
        reap(&mut self.child);
    }
}

fn quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

fn software_key() -> ClientKey {
    let signing = SigningKey::from_slice(&[29; 32]).unwrap();
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

struct Closed(std::sync::mpsc::Sender<()>);

impl Drop for Closed {
    fn drop(&mut self) {
        let _ = self.0.send(());
    }
}

struct SshStream {
    stream: UnixStream,
    _tunnel: Tunnel,
    // Fields drop in declaration order: stream, tunnel, then the signal.
    _closed: Closed,
}

impl Read for SshStream {
    fn read(&mut self, bytes: &mut [u8]) -> std::io::Result<usize> {
        self.stream.read(bytes)
    }
}

impl Write for SshStream {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.stream.write(bytes)
    }
    fn flush(&mut self) -> std::io::Result<()> {
        self.stream.flush()
    }
}

#[test]
#[ignore = "requires explicit HERDR_CORE_TEST_BINARY, HERDR_TEST_SSHD and HERDR_TEST_SSH_KEYGEN"]
fn a_real_core_over_ssh_streams_and_survives_detach() -> Result<()> {
    let fixture = PrivateCore::start()?;
    let key = software_key();
    let ssh = sshd::Sshd::start(&key.authorized_keys_line("herdr-core-test"), "ed25519")?;
    let target = Target::parse(&ssh.target()?)?;
    let Scan::New(keys) = herdr_ssh::scan(&target, &Pins::None, &AtomicBool::new(false))? else {
        anyhow::bail!("private sshd must require explicit trust");
    };
    let hosts = KnownHosts::new(ssh.directory().join("known_hosts"));
    hosts.trust(&target, &keys)?;
    let (stream, tunnel) = herdr_ssh::exec(
        &target,
        &key,
        &hosts.pins(&target)?,
        &fixture.ssh_command()?,
    )?;
    stream.set_read_timeout(Some(Duration::from_millis(10)))?;
    stream.set_write_timeout(Some(Duration::from_secs(5)))?;
    let (closed, observed) = std::sync::mpsc::channel();
    let core = Core::over(SshStream {
        stream,
        _tunnel: tunnel,
        _closed: Closed(closed),
    })?;
    ensure!(core.hello().generation == 1);
    ensure!(core.list(false)?.terminals.is_empty());
    let version = core.doc_set("ssh-bridge", &serde_json::json!({"transport": "ssh"}), 0)?;
    ensure!(core.doc_get("ssh-bridge")?.version == version);
    let argv = [
        "/bin/sh".to_owned(),
        "-c".to_owned(),
        "printf 'remote-core-byte-stream\\n'; sleep 30".to_owned(),
    ];
    let terminal = core.run(&Run {
        argv: &argv,
        name: Some("ssh-test"),
        ..Run::default()
    })?;
    let attachment = core.attach(&Attach {
        terminal: &terminal.address,
        cols: 80,
        rows: 24,
        ..Attach::default()
    })?;
    let deadline = Instant::now() + WAIT;
    let mut output = Vec::new();
    while !output
        .windows(b"remote-core-byte-stream".len())
        .any(|part| part == b"remote-core-byte-stream")
    {
        ensure!(Instant::now() < deadline, "no raw terminal output over SSH");
        let chunk = attachment.read(Duration::from_millis(100))?;
        ensure!(
            chunk.end.is_none(),
            "terminal ended before its output arrived"
        );
        output.extend(chunk.bytes);
    }
    drop(attachment);
    core.disconnect();
    observed.recv_timeout(WAIT)?;
    let local = Core::connect(&fixture.endpoint)?;
    ensure!(
        local
            .list(false)?
            .terminals
            .iter()
            .any(|record| record.address == terminal.address),
        "detaching the SSH viewer must leave its terminal running"
    );
    local.kill(&terminal.address)?;
    Ok(())
}
