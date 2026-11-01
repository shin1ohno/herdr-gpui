//! A private OpenSSH daemon shared by explicit integration harnesses.
//! Callers select both executables; only this fixture's exact child is stopped.
use super::sandbox::{self, Sandbox};
use anyhow::{Context as _, Result, ensure};
use std::{
    fs,
    net::{TcpListener, TcpStream},
    os::unix::fs::PermissionsExt as _,
    path::{Path, PathBuf},
    process::Child,
    thread,
    time::{Duration, Instant},
};

const DEADLINE: Duration = Duration::from_secs(15);

/// Declared before the sandbox so it is always killed and reaped first.
pub struct Sshd {
    child: Option<Child>,
    sandbox: Sandbox,
    port: u16,
}

impl Drop for Sshd {
    fn drop(&mut self) {
        sandbox::stop_children([&mut self.child]);
    }
}

pub fn executable(variable: &str) -> Result<PathBuf> {
    let path = PathBuf::from(
        std::env::var_os(variable)
            .with_context(|| format!("set {variable} to an explicit executable"))?,
    );
    ensure!(
        path.is_absolute() && path.is_file(),
        "{variable} must name an absolute file"
    );
    ensure!(
        fs::metadata(&path)?.permissions().mode() & 0o111 != 0,
        "{variable} must be executable"
    );
    Ok(path)
}

fn quoted(path: &Path) -> String {
    format!(
        "\"{}\"",
        path.display()
            .to_string()
            .replace('\\', "\\\\")
            .replace('"', "\\\"")
    )
}

impl Sshd {
    pub fn start(authorized_key: &str, host_algorithm: &str) -> Result<Self> {
        let daemon = executable("HERDR_TEST_SSHD")?;
        let keygen = executable("HERDR_TEST_SSH_KEYGEN")?;
        let sandbox = Sandbox::new();
        let host_key = sandbox.dir.join("host_key");
        let status = sandbox
            .command(keygen, "keygen.log")
            .args(["-q", "-t", host_algorithm, "-N", "", "-f"])
            .arg(&host_key)
            .status()?;
        ensure!(status.success(), "private host-key generation failed");
        let authorized = sandbox.dir.join("authorized_keys");
        fs::write(&authorized, format!("{authorized_key}\n"))?;
        fs::set_permissions(&authorized, fs::Permissions::from_mode(0o600))?;
        let listener = TcpListener::bind("127.0.0.1:0")?;
        let port = listener.local_addr()?.port();
        drop(listener);
        let config = sandbox.dir.join("sshd_config");
        fs::write(
            &config,
            format!(
                "Port {port}\nListenAddress 127.0.0.1\nHostKey {}\nPidFile {}\nAuthorizedKeysFile {}\n\
             PasswordAuthentication no\nKbdInteractiveAuthentication no\nUsePAM no\n\
             AuthenticationMethods publickey\nPubkeyAcceptedAlgorithms ecdsa-sha2-nistp256\n\
             PermitRootLogin prohibit-password\nStrictModes no\nUseDNS no\nLogLevel VERBOSE\n",
                quoted(&host_key),
                quoted(&sandbox.dir.join("sshd.pid")),
                quoted(&authorized)
            ),
        )?;
        let child = sandbox
            .command(daemon, "sshd.log")
            .args(["-D", "-e", "-f"])
            .arg(config)
            .spawn()?;
        let mut fixture = Self {
            child: Some(child),
            sandbox,
            port,
        };
        let deadline = Instant::now() + DEADLINE;
        loop {
            if TcpStream::connect(("127.0.0.1", port)).is_ok() {
                break;
            }
            let exited = fixture.child.as_mut().unwrap().try_wait()?;
            let log =
                || fs::read_to_string(fixture.sandbox.dir.join("sshd.log")).unwrap_or_default();
            ensure!(
                exited.is_none(),
                "private sshd exited: {exited:?}\n{}",
                log()
            );
            ensure!(
                Instant::now() < deadline,
                "private sshd startup timed out\n{}",
                log()
            );
            thread::sleep(Duration::from_millis(20));
        }
        Ok(fixture)
    }

    pub fn directory(&self) -> &Path {
        &self.sandbox.dir
    }

    pub fn target(&self) -> Result<String> {
        let user = std::env::var("HERDR_TEST_SSH_USER").or_else(|_| std::env::var("USER"))?;
        Ok(format!("{user}@127.0.0.1:{}", self.port))
    }
}
