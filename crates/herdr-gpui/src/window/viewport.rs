//! Keep app content outside platform-owned safe areas and the software keyboard.
//! The drawable remains full-sized; layout, painting and hit testing share the
//! smaller content area reported by GPUI for the current frame.
use gpui::{Edges, Pixels, Styled, Window, px};

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
