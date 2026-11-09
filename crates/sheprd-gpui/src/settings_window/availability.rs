//! Settings availability is presentation policy; saved values stay intact.
use super::Section;

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

/// Rows reviewed for iPad. New rows are hidden until classified here.
const IPAD_ROWS: &[&str] = &[
    // keep-sorted start
    "config-local",
    "config-reload",
    "fonts-all-row",
    "fonts-sidebar",
    "fonts-tabs",
    "fonts-terminal",
    "fonts-ui",
    "general-chat-full-width",
    "general-chat-glide",
    "general-chat-launcher",
    "general-chat-layout",
    "general-chat-width",
    "general-clipboard-position",
    "general-copied-toast",
    "general-feature-acp_chats",
    "general-feature-qa_menu",
    "general-feature-sidebar_hover_menu",
    "general-ipad-key",
    "general-ipad-key-replace",
    "general-language",
    "general-smooth-scroll",
    "general-window-controls",
    "indicators-edge-rail",
    "indicators-sets",
    "integrations-search",
    "keyboard-prefix",
    "keyboard-server",
    "keyboard-shortcuts",
    "keyboard-style-herdr-prefix",
    "keyboard-style-ipad",
    "notif-delay",
    "notif-finished",
    "notif-position",
    "notif-quiet-hours",
    "notif-show",
    "notif-volume",
    "sidebar-activity-bar",
    "sidebar-agents-placement",
    "sidebar-change-counts",
    "sidebar-device-footer",
    "sidebar-device-icons",
    "sidebar-host-colours",
    "sidebar-host-gap",
    "sidebar-indent",
    "sidebar-layout",
    "sidebar-ports-count",
    "sidebar-repository-icons",
    "sidebar-row-gap",
    "sidebar-row-padding",
    "sidebar-select",
    "sidebar-show-agents",
    "sidebar-show-agents-header",
    "sidebar-show-search",
    "status-keep-awake",
    "status-network-quality",
    "status-report-issue",
    "status-shortcuts",
    "status-theme",
    "tabs-agent-title",
    "tabs-confirm-pane",
    "tabs-confirm-tab",
    "tabs-confirm-workspace",
    "tabs-focus-follows-mouse",
    "tabs-layout",
    "theme-contrast",
    "theme-mode",
    "theme-padding-bottom",
    "theme-padding-left",
    "theme-padding-right",
    "theme-padding-top",
    "theme-side",
    // keep-sorted end
];

/// One policy for page rows and search results, including shared-file edits
/// that cannot reach the remote daemon's configuration over the tunnel.
pub(super) fn row(id: &str) -> bool {
    !ipad() || IPAD_ROWS.contains(&id)
}
