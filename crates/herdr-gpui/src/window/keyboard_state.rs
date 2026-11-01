use super::{HerdrWindow, viewport};
use gpui::{Context, Subscription, Window, px};

// A minimized hardware-keyboard assistant is about 55 pt; a docked keyboard
// is much taller. Floating keyboards do not occlude the bottom viewport.
const SOFT_MIN: f32 = 120.;

#[derive(Default)]
pub(crate) struct KeyboardState {
    hardware_seen: bool,
    /// Set around app-generated key dispatch (the key band and compact row).
    pub(crate) synthetic: bool,
}

impl KeyboardState {
    pub(crate) fn soft_keyboard_shown(window: &Window) -> bool {
        viewport::margins(window, px(0.)).bottom > px(SOFT_MIN)
    }

    pub(crate) fn hardware(&self, window: &Window) -> bool {
        self.hardware_seen && !Self::soft_keyboard_shown(window)
    }
}

impl HerdrWindow {
    /// A passive interceptor also sees keys consumed by bindings and prefix
    /// mode. App-wide callbacks must only update their own window's evidence.
    pub(crate) fn observe_keyboard(window: &Window, cx: &mut Context<Self>) -> Subscription {
        let own = window.window_handle().window_id();
        let view = cx.weak_entity();
        cx.intercept_keystrokes(move |_, window, cx| {
            if !crate::ipad_ui::active() || window.window_handle().window_id() != own {
                return;
            }
            let _ = view.update(cx, |this, cx| {
                let keyboard = &mut this.ipad.keyboard;
                if keyboard.hardware_seen
                    || keyboard.synthetic
                    || KeyboardState::soft_keyboard_shown(window)
                {
                    return;
                }
                keyboard.hardware_seen = true;
                if crate::ipad_ui::layout() {
                    cx.notify();
                }
            });
        })
    }
}
