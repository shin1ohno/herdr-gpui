//! Keep app content outside platform-owned safe areas and the software keyboard.
//! The drawable remains full-sized; layout, painting and hit testing share the
//! smaller content area reported by GPUI for the current frame.
use gpui::{Edges, Pixels, Styled, Window, px};

#[cfg(test)]
thread_local! {
    static WINDOW_CONTROLS: std::cell::Cell<Edges<Pixels>> = std::cell::Cell::new(Edges::default());
}

/// How far this window's top corners must be kept clear past its safe area,
/// per physical edge: iPadOS 26's window controls when they show (66 pt on
/// the left of a narrow window), else the window's corner allowance. Only
/// top-bar content pads by it; the rest of the window may run under.
pub(crate) fn window_controls(window: &Window) -> Edges<Pixels> {
    #[cfg(test)]
    {
        let _ = window;
        WINDOW_CONTROLS.get()
    }
    #[cfg(all(not(test), target_os = "ios"))]
    {
        herdr_uikit::window_control_insets(window.window_handle())
    }
    #[cfg(all(not(test), not(target_os = "ios")))]
    {
        let _ = window;
        Edges::default()
    }
}

#[cfg(test)]
pub(crate) fn simulate_window_controls(edges: Edges<Pixels>) -> SimulatedWindowControls {
    SimulatedWindowControls(WINDOW_CONTROLS.replace(edges))
}

#[cfg(test)]
pub(crate) struct SimulatedWindowControls(Edges<Pixels>);

#[cfg(test)]
impl Drop for SimulatedWindowControls {
    fn drop(&mut self) {
        WINDOW_CONTROLS.set(self.0);
    }
}

pub(crate) fn margins(window: &Window, extra: Pixels) -> Edges<Pixels> {
    let visible = window.fully_visible_bounds();
    let full = window.viewport_size();
    Edges {
        top: visible.top() + extra,
        right: full.width - visible.right() + extra,
        bottom: full.height - visible.bottom() + extra,
        left: visible.left() + extra,
    }
}

pub(crate) fn pad<E: Styled>(element: E, window: &Window) -> E {
    let insets = margins(window, px(0.));
    element
        .pt(insets.top)
        .pr(insets.right)
        .pb(insets.bottom)
        .pl(insets.left)
}
