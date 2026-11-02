//! iPad affordances use [`active`] for behavior, [`layout`] for geometry and
//! appearance, and target cfgs only for platform capabilities.
//!
//! Tests enable behavior by default, but keep desktop geometry unless
//! [`ipad_layout`] is held. [`desktop`] takes precedence over both gates.
//!
//! Theme roles (no iPad colors are hard-coded):
//! - Terminal: `background`; sidebar: `sidebar_background()`; bars: `surface`.
//! - Borders and floating parts: `active`; body: `foreground`; secondary: `subtext()`.
//! - Selection and focus: [`accent()`], with `text_on(accent())` for its text.
//!   Under the iPad layout, Nord uses `palette[6]`; other themes use `primary()`.
//! - Working/warnings: `palette[3]`; idle: `palette[6]`; done: `palette[2]`;
//!   blocked: `palette[1]`. Dot shapes retain the existing status meanings.

mod accent;
pub(crate) mod harness;
mod kbd;
pub(crate) mod metrics;
pub(crate) use accent::accent;
pub(crate) use kbd::kbd;

#[cfg(test)]
thread_local! {
    static DESKTOP: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    static IPAD_LAYOUT: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// True on iPadOS, and in tests unless [`desktop`] is in effect.
pub(crate) fn active() -> bool {
    #[cfg(test)]
    {
        !DESKTOP.get()
    }
    #[cfg(not(test))]
    {
        cfg!(target_os = "ios")
    }
}

/// iPad geometry and appearance; tests opt in without moving desktop fixtures.
pub(crate) fn layout() -> bool {
    #[cfg(test)]
    {
        IPAD_LAYOUT.get() && !DESKTOP.get()
    }
    #[cfg(not(test))]
    {
        cfg!(target_os = "ios")
    }
}

#[cfg(test)]
pub(crate) fn ipad_layout() -> IpadLayout {
    IpadLayout(IPAD_LAYOUT.replace(true))
}

#[cfg(test)]
pub(crate) struct IpadLayout(bool);

#[cfg(test)]
impl Drop for IpadLayout {
    fn drop(&mut self) {
        IPAD_LAYOUT.set(self.0);
    }
}

/// Takes the desktop side on this thread until the guard drops. GPUI's test
/// context renders and runs foreground tasks on the test's own thread.
#[cfg(test)]
pub(crate) fn desktop() -> Desktop {
    DESKTOP.set(true);
    Desktop
}

#[cfg(test)]
pub(crate) struct Desktop;

#[cfg(test)]
impl Drop for Desktop {
    fn drop(&mut self) {
        DESKTOP.set(false);
    }
}

#[cfg(test)]
mod tests;
