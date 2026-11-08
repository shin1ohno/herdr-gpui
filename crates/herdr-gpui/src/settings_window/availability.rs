//! Settings availability is presentation policy; saved values stay intact.
use super::{Section, catalog};

/// Tests opt into the iPad platform policy with the iPad layout harness.
pub(super) fn ipad() -> bool {
    #[cfg(test)]
    {
        crate::ipad_ui::layout()
    }
    #[cfg(not(test))]
    {
        cfg!(target_os = "ios")
    }
}

pub(super) fn section(section: Section) -> bool {
    if !ipad() {
        return true;
    }
    match section {
        Section::Plugins | Section::RemoteControl => false,
        #[cfg(feature = "cloud")]
        Section::CloudDevices => false,
        _ => true,
    }
}

/// One policy for page rows and search results, including shared-file edits
/// that cannot reach the remote daemon's configuration over the tunnel.
pub(super) fn row(id: &str) -> bool {
    if !ipad() {
        return true;
    }
    if catalog::CATALOG
        .iter()
        .any(|entry| entry.id == id && !section(entry.section))
    {
        return false;
    }
    !matches!(
        id,
        "theme-follow"
            | "theme-source"
            | "theme-backdrop"
            | "general-ports"
            | "general-checkpoints"
            | "general-copy-on-select"
            | "general-merged-worktrees"
            | "general-browser-skill"
            | "general-update-beta"
            | "status-usage"
            | "status-system-load"
            | "status-gpu"
            | "sidebar-gpu-load"
            | "tabs-herdr-2"
            | "tabs-position"
            | "tabs-hide-single"
            | "tabs-pane-history"
            | "notif-delivery"
            | "notif-sound"
            | "config-shared"
    )
}
