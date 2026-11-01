use super::keyboard_state::KeyboardState;

/// Window-local iPad data; subscriptions stay on HerdrWindow.
#[derive(Default)]
pub(crate) struct IpadState {
    // keep-sorted start
    pub(crate) keyboard: KeyboardState,
    // keep-sorted end
}
