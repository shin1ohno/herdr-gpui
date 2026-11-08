use super::{
    SheprdWindow,
    terminal::{input_area, input_cursor_bounds},
};
use gpui::*;
use sheprd_client::protocol::ClientPaneInputEvent;
use std::ops::Range;

#[cfg(any(target_os = "linux", windows))]
mod weak;
// The platform input handler for every view-backed text field. It is not named
// ElementInputHandler, so a `gpui::*` glob cannot silently replace it with
// GPUI's strong handler on Linux or Windows.
#[cfg(not(any(target_os = "linux", windows)))]
pub(crate) use gpui::ElementInputHandler as ViewInputHandler;
#[cfg(any(target_os = "linux", windows))]
pub(crate) use weak::WeakInputHandler as ViewInputHandler;

/// Which Option keys the keystroke being handled was made with. GPUI's
/// modifiers merge the two, so macOS's event in progress is asked; a key
/// event is handled while AppKit dispatches it, so that event is this one.
#[cfg(target_os = "macos")]
pub(crate) fn held_option_keys() -> crate::config::OptionKeys {
    use crate::config::OptionKeys;
    let Some(mtm) = objc2::MainThreadMarker::new() else {
        return OptionKeys::LEFT;
    };
    objc2_app_kit::NSApplication::sharedApplication(mtm)
        .currentEvent()
        .map_or(OptionKeys::LEFT, |event| {
            OptionKeys::from_device_flags(event.modifierFlags().0)
        })
}

/// Tells macOS's input method to drop its composition, once the update
/// that dropped the window's own has finished: the input method may call
/// back into the window, which must not be in use then.
#[cfg(target_os = "macos")]
fn discard_platform_composition(cx: &mut Context<SheprdWindow>) {
    cx.spawn(async move |_, _| {
        if let Some(mtm) = objc2::MainThreadMarker::new()
            && let Some(input) = objc2_app_kit::NSTextInputContext::currentInputContext(mtm)
        {
            input.discardMarkedText();
        }
    })
    .detach();
}

/// Elsewhere the platform keeps no composition the window must end.
#[cfg(not(target_os = "macos"))]
fn discard_platform_composition(_: &mut Context<SheprdWindow>) {}

impl SheprdWindow {
    /// A fresh user gesture may reopen a dismissed keyboard even when focus
    /// never changed. Copy mode keeps its navigation keys without opening it.
    pub(crate) fn request_terminal_keyboard(&self, window: &mut Window) {
        if self.terminal_accepts_text_input(window) {
            window.request_virtual_keyboard();
        }
    }

    fn terminal_accepts_text_input(&self, window: &Window) -> bool {
        self.menu.page.is_none()
            && self.focus.is_focused(window)
            && !self.copy_mode_active()
            && self.input_ready()
            && self.focused_input_target().is_some()
    }

    /// Drops the terminal's composition, here and in the platform's input
    /// method, when its pane or connection changes under it. Otherwise the
    /// next keystroke would carry on the dropped text into the new pane.
    pub(crate) fn discard_composition(&mut self, cx: &mut Context<Self>) {
        if self.marked.is_empty() {
            return;
        }
        self.marked.clear();
        self.marked_selection = None;
        #[cfg(feature = "integration-test")]
        {
            self.input_probe.compositions_discarded += 1;
        }
        discard_platform_composition(cx);
    }
}

/// Whether text focus survives while the focused pane waits for the daemon to
/// resize its surface. Only iPadOS resizes the pane when the software keyboard
/// appears; resigning there would hide the keyboard and resize it back. Tests
/// build this policy on every platform.
const RETAIN_TEXT_FOCUS_DURING_RESIZE: bool = cfg!(any(target_os = "ios", test));

/// Whether a focused terminal takes text: copy mode always does, a pane ready
/// for input does, and with `retain_during_resize` a pane still waiting on the
/// same target after a keyboard resize does. Sending keeps `send`'s readiness
/// checks either way, and a fresh summon still requires `input_ready`.
pub(crate) fn terminal_text_focus(
    retain_during_resize: bool,
    focused: bool,
    copy_mode: bool,
    waiting_on_same_target: bool,
    accepts_input: bool,
) -> bool {
    (focused && (copy_mode || (retain_during_resize && waiting_on_same_target))) || accepts_input
}

/// Elsewhere Alt has no sides that matter: it always reaches the pane.
#[cfg(not(target_os = "macos"))]
pub(crate) fn held_option_keys() -> crate::config::OptionKeys {
    crate::config::OptionKeys::LEFT
}

impl EntityInputHandler for SheprdWindow {
    fn accepts_text_input(&self, window: &mut Window, _: &mut Context<Self>) -> bool {
        if self.menu.page.is_some() {
            return self.menu.input.is_some() && self.menu.focus.is_focused(window);
        }
        terminal_text_focus(
            RETAIN_TEXT_FOCUS_DURING_RESIZE,
            self.focus.is_focused(window),
            self.copy_mode_active(),
            self.pending_input_keeps_text_focus(),
            self.terminal_accepts_text_input(window),
        )
    }

    fn text_for_range(
        &mut self,
        range: Range<usize>,
        adjusted: &mut Option<Range<usize>>,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<String> {
        if self.menu.page.is_some() {
            let input = self.menu.input.as_ref()?;
            let range = input.range_from_utf16(range);
            *adjusted = Some(input.to_utf16(range.clone()));
            return Some(input.text[range].to_owned());
        }
        let text: Vec<u16> = self.marked.encode_utf16().collect();
        let range = range.start.min(text.len())..range.end.min(text.len());
        *adjusted = Some(range.clone());
        Some(String::from_utf16_lossy(&text[range]))
    }
    fn selected_text_range(
        &mut self,
        _: bool,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<UTF16Selection> {
        if self.menu.page.is_some() {
            let input = self.menu.input.as_ref()?;
            return Some(UTF16Selection {
                range: input.to_utf16(input.selection.clone()),
                reversed: input.reversed,
            });
        }
        let end = self.marked.encode_utf16().count();
        let range = self
            .marked_selection
            .clone()
            .filter(|range| range.start <= range.end && range.end <= end)
            .unwrap_or(end..end);
        Some(UTF16Selection {
            range,
            reversed: false,
        })
    }
    fn marked_text_range(&self, _: &mut Window, _: &mut Context<Self>) -> Option<Range<usize>> {
        if self.menu.page.is_some() {
            let input = self.menu.input.as_ref()?;
            return input.marked.clone().map(|range| input.to_utf16(range));
        }
        (!self.marked.is_empty()).then(|| 0..self.marked.encode_utf16().count())
    }
    fn unmark_text(&mut self, _: &mut Window, cx: &mut Context<Self>) {
        if let Some(input) = self.menu.input.as_mut() {
            input.unmark();
        }
        self.marked.clear();
        self.marked_selection = None;
        cx.notify();
    }
    fn replace_text_in_range(
        &mut self,
        range: Option<Range<usize>>,
        text: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.shift_taps.cancel();
        if self.menu.page.is_some() {
            if let Some(input) = self.menu.input.as_mut() {
                input.replace(range, text, false, None);
                cx.notify();
            }
            return;
        }
        if self.ipad_latched_text(text, window, cx) {
            self.marked.clear();
            self.marked_selection = None;
            return;
        }
        if self.copy_mode_active() {
            self.copy_mode_text(text, window, cx);
            return;
        }
        #[cfg(feature = "integration-test")]
        {
            self.input_probe.text += 1;
        }
        self.marked.clear();
        self.marked_selection = None;
        if !text.is_empty() {
            self.send(ClientPaneInputEvent::TextCommit(text.into()), cx);
        }
        cx.notify();
    }
    fn replace_and_mark_text_in_range(
        &mut self,
        range: Option<Range<usize>>,
        text: &str,
        selected: Option<Range<usize>>,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.shift_taps.cancel();
        // Nothing reaches a pane until the composition commits, but this is
        // the user typing all the same.
        self.note_input();
        if self.menu.page.is_some() {
            if let Some(input) = self.menu.input.as_mut() {
                input.replace(range, text, true, selected);
                cx.notify();
            }
            return;
        }
        // Copy mode has no composition: a key it does not know types nothing.
        // While the focused pane waits out a keyboard resize, composition stays
        // local (marked text is never sent), so the input method keeps it.
        let composing = self.input_ready()
            || (RETAIN_TEXT_FOCUS_DURING_RESIZE && self.pending_input_keeps_text_focus());
        if !composing || self.copy_mode_active() {
            return;
        }
        self.marked = text.into();
        // The IME's caret, or the clause it is converting, so it asks for
        // the candidate window's place there rather than after the text.
        self.marked_selection = selected;
        cx.notify();
    }
    fn bounds_for_range(
        &mut self,
        range: Range<usize>,
        _: Bounds<Pixels>,
        window: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<Bounds<Pixels>> {
        if self.menu.page.is_some() {
            return self.menu.input.as_ref()?.range_bounds(range);
        }
        let surface = self.live.surface.as_deref();
        let cell_height = self.config.terminal.line_height();
        let mut cursor =
            input_cursor_bounds(surface, self.bounds.origin, self.cell_width, cell_height);
        cursor.origin.y += self.ime_shift();
        Some(self.painter.borrow().composition_bounds(
            &self.marked,
            range,
            cursor,
            input_area(surface, self.bounds, self.cell_width, cell_height),
            &self.config.terminal.font(),
            window,
        ))
    }
    fn character_index_for_point(
        &mut self,
        point: Point<Pixels>,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<usize> {
        if self.menu.page.is_some() {
            let input = self.menu.input.as_ref()?;
            let index = input.index_at(point)?;
            return Some(input.to_utf16(index..index).start);
        }
        None
    }
}

/// The terminal's platform input handler. A terminal repeats a held key, so
/// it opts out of macOS press-and-hold, which would otherwise swallow the
/// repeats and open the accent picker. Menu text fields keep the picker.
pub(crate) struct TerminalInputHandler<V: EntityInputHandler = SheprdWindow> {
    inner: ViewInputHandler<V>,
    press_and_hold: bool,
}

impl<V: EntityInputHandler> TerminalInputHandler<V> {
    pub(crate) fn new(bounds: Bounds<Pixels>, view: Entity<V>, menu_open: bool) -> Self {
        Self {
            inner: ViewInputHandler::new(bounds, view),
            press_and_hold: menu_open,
        }
    }
}

impl<V: EntityInputHandler> InputHandler for TerminalInputHandler<V> {
    fn accepts_text_input(&mut self, window: &mut Window, cx: &mut App) -> bool {
        self.inner.accepts_text_input(window, cx)
    }

    fn prefers_ime_for_printable_keys(&mut self, window: &mut Window, cx: &mut App) -> bool {
        self.inner.prefers_ime_for_printable_keys(window, cx)
    }

    fn selected_text_range(
        &mut self,
        ignore_disabled_input: bool,
        window: &mut Window,
        cx: &mut App,
    ) -> Option<UTF16Selection> {
        self.inner
            .selected_text_range(ignore_disabled_input, window, cx)
    }
    fn marked_text_range(&mut self, window: &mut Window, cx: &mut App) -> Option<Range<usize>> {
        self.inner.marked_text_range(window, cx)
    }
    fn text_for_range(
        &mut self,
        range_utf16: Range<usize>,
        adjusted_range: &mut Option<Range<usize>>,
        window: &mut Window,
        cx: &mut App,
    ) -> Option<String> {
        self.inner
            .text_for_range(range_utf16, adjusted_range, window, cx)
    }
    fn replace_text_in_range(
        &mut self,
        replacement_range: Option<Range<usize>>,
        text: &str,
        window: &mut Window,
        cx: &mut App,
    ) {
        self.inner
            .replace_text_in_range(replacement_range, text, window, cx);
    }
    fn replace_and_mark_text_in_range(
        &mut self,
        range_utf16: Option<Range<usize>>,
        new_text: &str,
        new_selected_range: Option<Range<usize>>,
        window: &mut Window,
        cx: &mut App,
    ) {
        self.inner.replace_and_mark_text_in_range(
            range_utf16,
            new_text,
            new_selected_range,
            window,
            cx,
        );
    }
    fn unmark_text(&mut self, window: &mut Window, cx: &mut App) {
        self.inner.unmark_text(window, cx);
    }
    fn bounds_for_range(
        &mut self,
        range_utf16: Range<usize>,
        window: &mut Window,
        cx: &mut App,
    ) -> Option<Bounds<Pixels>> {
        self.inner.bounds_for_range(range_utf16, window, cx)
    }
    fn character_index_for_point(
        &mut self,
        point: Point<Pixels>,
        window: &mut Window,
        cx: &mut App,
    ) -> Option<usize> {
        self.inner.character_index_for_point(point, window, cx)
    }
    fn apple_press_and_hold_enabled(&mut self) -> bool {
        self.press_and_hold
    }
}

#[cfg(test)]
mod tests;
