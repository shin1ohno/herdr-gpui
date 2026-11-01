//! Why adding, saving, removing, or discovering a device failed.

#[derive(Debug, thiserror::Error)]
pub enum Error {
    // keep-sorted start
    #[error(
        "This host is already being added, in another Add Device dialog or a \u{201c}Set up\u{201d} workspace that is still running. Finish or close it, then try again."
    )]
    Adding,
    #[cfg(target_os = "ios")]
    #[error(transparent)]
    Bonjour(#[from] super::discover::bonjour::Error),
    #[error("This host and session are already saved as \u{201c}{0}\u{201d}.")]
    Exists(String),
    #[error("Device names are at most 128 bytes, without control characters.")]
    InvalidLabel,
    #[error(
        "Session names use letters, digits, dots, underscores, or hyphens (at most 64 bytes), excluding '.' and '..'."
    )]
    InvalidSession,
    #[error("Enter an SSH target such as user@hostname or an SSH alias.")]
    InvalidTarget,
    #[error("The setup paths must be UTF-8.")]
    NonUtf8SetupPath,
    #[error("This device has no saved profile.")]
    NoProfile,
    #[error("Herdr UI has no state directory.")]
    NoStateDirectory,
    #[error("Removing the device failed ({status}){}", if detail.is_empty() { String::new() } else { format!(": {detail}") })]
    Remove {
        status: std::process::ExitStatus,
        detail: String,
    },
    #[error("Renaming the device failed ({status}){}", if detail.is_empty() { String::new() } else { format!(": {detail}") })]
    Rename {
        status: std::process::ExitStatus,
        detail: String,
    },
    /// Herdr's `machine add` refused to save the host until its running
    /// server is restarted, which it only does after a confirmation.
    #[error(
        "The Herdr server on this host was started in a way that may not survive a dropped SSH connection, so Herdr has to restart it before saving the device. Restarting stops the panes running there."
    )]
    ServerRestart,
    #[error("Saving the device failed ({status}){}", if detail.is_empty() { String::new() } else { format!(": {detail}") })]
    Setup {
        status: std::process::ExitStatus,
        detail: String,
    },
    #[error("Saving the device timed out")]
    SetupTimeout,
    #[error("Tailscale returned an unreadable status: {0}")]
    TailscaleJson(#[source] serde_json::Error),
    #[error("Tailscale did not answer in time")]
    TailscaleTimeout,
    #[cfg_attr(
        target_os = "ios",
        error("{}", Self::IPADOS_TERMINAL_SETUP_UNAVAILABLE)
    )]
    #[cfg_attr(
        not(target_os = "ios"),
        error("Setting up a device in a terminal is unavailable on Windows.")
    )]
    TerminalSetupUnavailable,
    // keep-sorted end
}

impl Error {
    /// What [`Self::TerminalSetupUnavailable`] says on iPadOS, named so tests
    /// on any host check `error-devices-setup-ipad` against it. Like every
    /// `#[error]`, it is the log's text; the window shows the message.
    #[cfg(any(target_os = "ios", test))]
    pub(crate) const IPADOS_TERMINAL_SETUP_UNAVAILABLE: &str =
        // i18n: data
        "Setting up a device in a local terminal is unavailable on iPadOS.";
}
