//! Touch keys stay on the existing keyboard and committed-text input paths.
mod band;
mod catalog;
#[path = "window/compact.rs"]
mod compact;
mod latch;

#[cfg(test)]
mod tests;

pub(crate) use compact::OverlaySelection;
pub(crate) use latch::Latch;

use crate::HerdrWindow;
use catalog::{Binding, Key};
use gpui::{App, Context, EntityInputHandler, Keystroke, WeakEntity, Window};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Page {
    // keep-sorted start
    At(usize),
    Next,
    Previous,
    // keep-sorted end
}

impl Page {
    pub(crate) fn index(self, current: usize, count: usize) -> Option<usize> {
        if count == 0 || current >= count {
            return None;
        }
        match self {
            Self::Next => Some((current + 1) % count),
            Self::Previous => Some((current + count - 1) % count),
            Self::At(index) => (index < count).then_some(index),
        }
    }
}

/// Dispatch after the input/button callback returns its entity lease. Both
/// the prefix interceptor and hardware observer update that same window.
pub(crate) fn defer_key(
    view: WeakEntity<HerdrWindow>,
    key: Keystroke,
    window: &Window,
    cx: &mut App,
) {
    window.defer(cx, move |window, cx| {
        let Ok(previous) = view.update(cx, |this, _| {
            std::mem::replace(&mut this.ipad.keyboard.synthetic, true)
        }) else {
            return;
        };
        window.dispatch_keystroke(key, cx);
        let _ = view.update(cx, |this, cx| {
            this.ipad.keyboard.synthetic = previous;
            cx.notify();
        });
    });
}

impl HerdrWindow {
    pub(crate) fn ipad_latched_text(
        &mut self,
        text: &str,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> bool {
        if !crate::ipad_ui::active() {
            return false;
        }
        let Some(key) = self.ipad.latch.take(text) else {
            return false;
        };
        defer_key(cx.weak_entity(), key, window, cx);
        cx.notify();
        true
    }

    fn ipad_key(&mut self, key: Key, window: &mut Window, cx: &mut Context<Self>) {
        if !crate::ipad_ui::active() {
            return;
        }
        match key.binding() {
            Binding::Keystroke(key) => {
                if let Ok(key) = Keystroke::parse(key) {
                    defer_key(cx.weak_entity(), key, window, cx);
                }
            }
            Binding::Text(text) => {
                let owner = cx.weak_entity();
                window.defer(cx, move |window, cx| {
                    let Some(owner) = owner.upgrade() else { return };
                    let core = owner.read(cx).ipad_core_text_target(window, cx);
                    if let Some(core) = core {
                        core.update(cx, |view, cx| {
                            view.replace_text_in_range(None, text, window, cx);
                        });
                    } else if owner.read(cx).focus.is_focused(window) {
                        owner.update(cx, |view, cx| {
                            view.replace_text_in_range(None, text, window, cx);
                        });
                    }
                });
            }
            Binding::Control => self.ipad.latch.toggle_control(),
            Binding::Alt => self.ipad.latch.toggle_alt(),
            Binding::Paste => {
                window.defer(cx, |window, cx| {
                    window.dispatch_action(Box::new(crate::actions::Paste), cx);
                });
            }
            Binding::Hide => {
                #[cfg(target_os = "ios")]
                window.dismiss_virtual_keyboard();
            }
            Binding::More => {
                let now = crate::motion::now();
                let open = !self.ipad.more_keys.is_open();
                self.ipad.more_keys.set(open, now, cfg!(test));
            }
        }
        cx.notify();
    }
}
