use super::keyboard_state::KeyboardState;

/// Window-local iPad data; subscriptions stay on SheprdWindow.
#[derive(Default)]
pub(crate) struct IpadState {
    // keep-sorted start
    pub(crate) compact_font: Option<f32>,
    pub(crate) font_update_pending: bool,
    pub(crate) guide: super::shortcut_guide::Guide,
    pub(crate) keyboard: KeyboardState,
    pub(crate) latch: crate::ipad_keys::Latch,
    pub(crate) more_keys: crate::motion::Toggle,
    pub(crate) overlay_open: crate::motion::Toggle,
    pub(crate) overlay_selection: Option<crate::ipad_keys::OverlaySelection>,
    pub(crate) pane_zoom:
        std::collections::HashMap<crate::browser::TabId, Option<sheprd_backend::TerminalId>>,
    // keep-sorted end
}
