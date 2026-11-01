//! What the iPad's in-process SSH client reports, read as this crate's types
//! (docs/ios.md, "SSH transport"). The desktop classifies a failed `ssh` from
//! its exit code and stderr text; the in-process client reports typed exits
//! instead, so a class here comes only from those types. A remote program's
//! stderr, which anyone controlling the host can write, never names one.
//!
//! Pure functions only, so they run in this crate's tests on any Unix host.
use super::{HostProbe, SshFailure};
use herdr_ssh::{Failure, TunnelExit};

/// A working Herdr 1 remains the preferred path. When it cannot connect,
/// discover Herdr 2 without losing whether Herdr 1 needs an update.
///
/// The Herdr 2 check is a second, optional connection. When it fails, the
/// Herdr 1 result stands, so the user still gets its install or update
/// advice rather than a probe error.
pub(super) fn with_core_probe(
    probe: HostProbe,
    core_available: impl FnOnce() -> crate::Result<bool>,
) -> HostProbe {
    if !matches!(probe, HostProbe::Missing | HostProbe::Outdated) {
        return probe;
    }
    let available = core_available().unwrap_or_else(|error| {
        tracing::warn!(
            category = "probe",
            ?error,
            "Herdr 2 check failed; keeping the Herdr 1 result"
        );
        false
    });
    match (probe, available) {
        (HostProbe::Outdated, true) => HostProbe::OutdatedCoreAvailable,
        (_, true) => HostProbe::CoreAvailable,
        (probe, false) => probe,
    }
}

/// Preserve a remote command's status without letting an out-of-range SSH
/// exit status wrap to success in Unix's eight-bit process status encoding.
pub(super) fn remote_status(status: Option<u32>) -> std::process::ExitStatus {
    use std::os::unix::process::ExitStatusExt;
    match status {
        Some(code) => std::process::ExitStatus::from_raw((code.min(255) as i32) << 8),
        None => std::process::ExitStatus::from_raw(1),
    }
}

/// The class a failed SSH layer reports as. A host key that is unknown or
/// changed needs the user, so a reconnect never retries it soon and never
/// asks about it.
pub(super) fn ssh_failure(failure: Failure) -> SshFailure {
    match failure {
        Failure::HostKeyUnknown | Failure::HostKeyChanged => SshFailure::HostKey,
        Failure::Auth => SshFailure::Auth,
        Failure::Unreachable | Failure::Timeout => SshFailure::Unreachable,
        // The caller that cancelled already knows; nothing here needs the user.
        Failure::Protocol | Failure::Cancelled => SshFailure::Other,
    }
}

/// Why a tunnel closed before its bridge was ready. A remote program's exit
/// is read only by its status: 127 is the bridge script finding no Herdr,
/// as on the desktop, and anything else names no SSH class.
pub(super) fn exit_failure(exit: &TunnelExit) -> SshFailure {
    match exit {
        TunnelExit::Ssh(failure) => ssh_failure(*failure),
        TunnelExit::Remote {
            exit_status: Some(127),
            ..
        } => SshFailure::HerdrMissing,
        TunnelExit::Remote { .. } => SshFailure::Other,
    }
}

/// A host key algorithm as `ssh-keygen -l` names its type, with the key's
/// size where the algorithm fixes it. RSA keys vary, and the in-process
/// client reports no size for them.
pub(super) fn key_kind(algorithm: &str) -> Option<(&'static str, u32)> {
    Some(match algorithm {
        "ssh-ed25519" => ("ED25519", 256),
        "ecdsa-sha2-nistp256" => ("ECDSA", 256),
        "ecdsa-sha2-nistp384" => ("ECDSA", 384),
        "ecdsa-sha2-nistp521" => ("ECDSA", 521),
        "ssh-rsa" | "rsa-sha2-256" | "rsa-sha2-512" => ("RSA", 0),
        _ => return None,
    })
}

/// Longest device name kept in a key comment.
const NAME_LIMIT: usize = 48;

/// The comment on this iPad's `authorized_keys` line, `herdr-ipad-<device>`,
/// so the user can tell the line apart and remove it later. The name is one
/// word of ASCII letters, digits, `-`, `.`, and `_`: anything else becomes
/// `-`, and without a name the comment is `herdr-ipad`.
pub(super) fn key_comment(device: &str) -> String {
    let device = device.trim().trim_end_matches(".local");
    let name: String = device
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '-' | '.' | '_') {
                c
            } else {
                '-'
            }
        })
        .take(NAME_LIMIT)
        .collect();
    let name = name.trim_matches('-');
    if name.is_empty() {
        "herdr-ipad".to_owned()
    } else {
        format!("herdr-ipad-{name}")
    }
}

#[cfg(test)]
mod tests;
