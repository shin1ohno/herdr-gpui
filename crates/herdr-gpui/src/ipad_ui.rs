//! Whether the window offers the iPad's touch affordances: menus a long press
//! opens on rows that have none on the desktop, a placeholder that offers to
//! reconnect, and dialogs that wrap and scroll inside a small window.
//!
//! Headless tests take the iPad side, so its paths run on a desktop host.
//! [`desktop`] switches the calling thread to the desktop side, so the
//! behaviour these gates keep for the desktop is tested as well.

#[cfg(test)]
thread_local! {
    static DESKTOP: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
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
