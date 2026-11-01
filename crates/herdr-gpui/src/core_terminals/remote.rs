//! The remote core a saved tab belongs to. `hc connect` is the core's own
//! stdin/stdout bridge; its SSH transport shares the app's device key and pins.

use herdr_client::ConnectTarget;
use serde::{Deserialize, Serialize};

/// Both fields identify the core namespace: `t1` on another host or session
/// is another terminal. Old tabs without this record remain local-only.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct Remote {
    session: String,
    target: String,
}

impl Remote {
    pub(super) fn from_target(target: &ConnectTarget) -> Option<Self> {
        match target {
            ConnectTarget::Ssh { target, session } => Some(Self {
                session: session.clone(),
                target: target.clone(),
            }),
            _ => None,
        }
    }

    pub(super) fn matches_target(origin: Option<&Self>, target: &ConnectTarget) -> bool {
        match (origin, target) {
            (Some(origin), ConnectTarget::Ssh { target, session }) => {
                origin.target == *target && origin.session == *session
            }
            (None, ConnectTarget::Ssh { .. }) | (Some(_), _) => false,
            (None, _) => true,
        }
    }

    #[cfg(any(target_os = "ios", test))]
    fn command(&self) -> Result<String, super::Error> {
        Ok(herdr_client::core_connect_command(&self.session)?)
    }

    #[cfg(target_os = "ios")]
    pub(super) fn connect(&self) -> Result<herdr_core_client::CoreBackend, super::Error> {
        use std::{sync::Arc, time::Duration};
        let (stream, tunnel) = herdr_client::exec_ssh_stream(&self.target, &self.command()?)?;
        stream.set_read_timeout(Some(Duration::from_millis(10)))?;
        stream.set_write_timeout(Some(Duration::from_secs(5)))?;
        let tunnel = Arc::new(tunnel);
        let connection = herdr_core_client::Core::over(SshStream {
            stream,
            _tunnel: Arc::clone(&tunnel),
        });
        match connection {
            Ok(core) => Ok(herdr_core_client::CoreBackend::new(
                core,
                super::hub::SCROLLBACK,
            )),
            Err(error) => match tunnel.wait_exit(Duration::ZERO) {
                Some(herdr_ssh::TunnelExit::Ssh(failure)) => Err(super::Error::Ssh(failure)),
                Some(herdr_ssh::TunnelExit::Remote { exit_status, .. }) => {
                    Err(super::Error::RemoteCommand { exit_status })
                }
                None => Err(error.into()),
            },
        }
    }
}

#[cfg(target_os = "ios")]
struct SshStream {
    stream: herdr_client::Stream,
    // The worker retains the tunnel, including during the plain-mode hello.
    _tunnel: std::sync::Arc<herdr_ssh::Tunnel>,
}

#[cfg(target_os = "ios")]
impl std::io::Read for SshStream {
    fn read(&mut self, bytes: &mut [u8]) -> std::io::Result<usize> {
        std::io::Read::read(&mut self.stream, bytes)
    }
}

#[cfg(target_os = "ios")]
impl std::io::Write for SshStream {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        std::io::Write::write(&mut self.stream, bytes)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        std::io::Write::flush(&mut self.stream)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests;
