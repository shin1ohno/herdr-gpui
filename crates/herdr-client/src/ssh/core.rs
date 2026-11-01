//! Read-only discovery and the explicit Herdr 2 exec command share candidates.
//! Neither command installs or upgrades a core or changes a Herdr 1 session.
use crate::{Result, session_socket};
use std::path::Path;

const CANDIDATES: &str = r#"for path in "$(command -v hc 2>/dev/null || true)" "$(command -v herdr-core 2>/dev/null || true)" "$HOME/.local/bin/hc" "$HOME/.local/bin/herdr-core" "$HOME/.cargo/bin/hc" "$HOME/.cargo/bin/herdr-core" "/opt/homebrew/bin/hc" "/opt/homebrew/bin/herdr-core" "/usr/local/bin/hc" "/usr/local/bin/herdr-core"; do
    if [ -n "$path" ] && [ -f "$path" ] && [ -x "$path" ]; then"#;

/// The remote POSIX command for an explicitly requested Herdr 2 terminal.
/// The caller owns transport, framing and the guard for its lifetime.
pub fn core_connect_command(session: &str) -> Result<String> {
    session_socket(Path::new(""), session)?;
    let script = format!(
        "{CANDIDATES}\n        exec \"$path\" connect --session {}\n    fi\ndone\nexit 127",
        super::quote(session)
    );
    Ok(format!("/bin/sh -c {}", super::quote(&script)))
}

#[cfg(target_os = "ios")]
pub(super) fn probe_command() -> String {
    let script = format!(
        "{CANDIDATES}\n        printf '%s\\n' 'herdr-core-available:1'\n        exit 0\n    fi\ndone\nexit 1"
    );
    format!("/bin/sh -c {}", super::quote(&script))
}

#[cfg(test)]
mod tests;
