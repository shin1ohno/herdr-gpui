use gpui::Keystroke;
use std::{cell::Cell, rc::Rc};

#[derive(Clone, Default)]
pub(crate) struct Latch {
    state: Rc<Cell<State>>,
}

#[derive(Clone, Copy, Default)]
struct State {
    alt: bool,
    control: bool,
}

impl Latch {
    pub(crate) fn toggle_control(&self) {
        let state = self.state.get();
        self.state.set(State {
            control: !state.control,
            ..state
        });
    }

    pub(super) fn toggle_alt(&self) {
        let state = self.state.get();
        self.state.set(State {
            alt: !state.alt,
            ..state
        });
    }

    pub(super) fn control(&self) -> bool {
        self.state.get().control
    }

    pub(super) fn alt(&self) -> bool {
        self.state.get().alt
    }

    /// Composition updates never enter here. A multi-character commit stays
    /// intact; only a single committed scalar consumes the one-shot modifiers.
    pub(crate) fn take(&self, text: &str) -> Option<Keystroke> {
        let state = self.state.get();
        if !state.alt && !state.control {
            return None;
        }
        let mut chars = text.chars();
        let c = chars.next()?;
        if chars.next().is_some() || c.is_control() {
            return None;
        }
        let mut key = crate::keymap::parse_keystroke(if c == ' ' { "space" } else { text }).ok()?;
        key.modifiers.control = state.control;
        key.modifiers.alt = state.alt;
        key.key_char = None;
        self.state.set(State::default());
        Some(key)
    }
}
