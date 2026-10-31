//! `user@host[:port]`, parsed once at the boundary and then shared by scan,
//! trust, connect, and the device list (docs/ios.md, "Targets").

use crate::Result;

/// A parsed SSH target. Host names are ASCII lower-case without a trailing
/// dot; IPv6 addresses come only as `[addr]:port`, without zone ids.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Target {
    pub user: String,
    pub host: String,
    pub port: u16,
}

impl Target {
    /// Parses and normalizes `user@host`, `user@host:port`, or
    /// `user@[v6addr]:port`. The user is required.
    pub fn parse(_text: &str) -> Result<Self> {
        todo!("ssh stream")
    }

    /// The name host keys are pinned under: `host` on port 22, otherwise
    /// `[host]:port`, as OpenSSH writes it.
    pub fn known_hosts_name(&self) -> String {
        todo!("ssh stream")
    }
}
