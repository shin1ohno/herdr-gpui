//! An SSH client that runs inside the process, for platforms that cannot start
//! `ssh` (iPadOS). It does one thing Herdr needs: run a command on a host over
//! an exec channel and expose that channel's standard streams as one end of a
//! `UnixStream::pair()`, so `sheprd-client` can read the remote bridge exactly
//! as it reads an `ssh` child's (docs/ios.md, "SSH transport").
//!
//! russh does the protocol on one process-wide current-thread tokio runtime
//! that lives on its own thread; nothing async crosses this crate's API. The
//! client key is held outside the crate (the Secure Enclave on iPad) and
//! reached through a signing closure. Host keys are pinned in an OpenSSH
//! `known_hosts` file that this crate reads and writes itself, because a pin
//! of any key type must make every other key a changed key.
//!
//! A caller that must know a tunnel's local work has ended before it starts
//! another (a reader that must never run twice at once) stops the tunnel and
//! waits for its [`Teardown`]: a [`TeardownReceipt`] names the tunnel it
//! ends and is only given once russh's session task has been joined (module
//! `teardown` lists what it shows and what it does not).
//!
//! The crate builds on every Unix target, so its integration tests run on
//! macOS against a private `sshd` (`just test-ssh`).

#![cfg(unix)]

// keep-sorted start
mod classify;
mod client_key;
mod config;
mod error;
mod exec;
mod handler;
mod hooks;
mod host_key;
mod known_hosts;
mod runtime;
mod target;
pub mod teardown;
#[cfg(test)]
mod test_host;
mod transport;
mod tunnel;
// keep-sorted end

pub use client_key::{ClientKey, SignError};
pub use error::{Error, Failure, Result};
pub use host_key::HostKey;
pub use known_hosts::{KnownHosts, Pins};
pub use target::Target;
pub use teardown::{Teardown, TeardownReceipt, TunnelId};
pub use tunnel::{Scan, Tunnel, TunnelExit, exec, scan};
