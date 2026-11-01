//! Where a footer menu, such as the device picker, opens beside the control
//! that opened it: the room above or below it within the visible bounds, and
//! its start edge, kept inside them as a software keyboard shrinks them.

use super::super::{MENU_MARGIN, Page};
use crate::HerdrWindow;
use crate::direction::Logical;
use gpui::{prelude::*, *};

impl HerdrWindow {
    /// Places a footer menu beside its anchor, `lift` short of it while it
    /// slides into place.
    pub(super) fn anchor_footer_panel(
        &self,
        panel: Stateful<Div>,
        viewport: Size<Pixels>,
        visible: Bounds<Pixels>,
        page: Page,
        lift: Pixels,
    ) -> Stateful<Div> {
        let (room, above) = self.footer_room(visible);
        let list = page == Page::Devices;
        let width = if list {
            super::super::devices::MENU_WIDTH
        } else {
            180.
        };
        // From the window's start edge, which right to left is its right.
        let anchor = match crate::direction::is_rtl() {
            false => self.menu.anchor.x,
            true => viewport.width - self.menu.anchor.x,
        };
        let start = match page {
            Page::Devices => anchor,
            _ => px(56.),
        };
        let width = px(width).min((visible.size.width - px(16.)).max(px(0.)));
        let (near, far) = if crate::direction::is_rtl() {
            (
                viewport.width - visible.right(),
                viewport.width - visible.left(),
            )
        } else {
            (visible.left(), visible.right())
        };
        let start = start
            .max(near + px(8.))
            .min((far - width - px(8.)).max(near));
        let panel = panel.absolute().inset_s(start).w(width);
        if above {
            panel
                .bottom(
                    (viewport.height - self.menu.anchor.y
                        + px(if list {
                            super::super::devices::MENU_GAP
                        } else {
                            12.
                        }))
                    .max(px(MENU_MARGIN))
                        - lift,
                )
                .max_h(room)
        } else {
            panel.top(self.menu.anchor.y + px(12.) - lift).max_h(room)
        }
    }

    /// The room a footer menu has, and whether it opens above its anchor,
    /// as it does from the sidebar footer at the window's foot.
    pub(super) fn footer_room(&self, visible: Bounds<Pixels>) -> (Pixels, bool) {
        let chrome = visible.top() + px(crate::titlebar::HEIGHT);
        let band = (visible.bottom() - chrome - px(2. * MENU_MARGIN)).max(px(0.));
        let room = |side: Pixels| side.clamp(px(0.), band).max(px(60.)).min(band);
        let above = room(self.menu.anchor.y - px(12. + MENU_MARGIN) - chrome);
        let below = room(visible.bottom() - self.menu.anchor.y - px(12. + MENU_MARGIN));
        if above >= below {
            (above, true)
        } else {
            (below, false)
        }
    }
}
