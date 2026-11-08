use super::*;
use core::prelude::v1::test;

#[gpui::test]
fn compact_strip_keeps_close_and_hint_inside_tabs_at_ipad_widths(cx: &mut TestAppContext) {
    let _ipad = crate::ipad_ui::ipad_layout();
    let (view, cx) = four_tabs(cx, false);
    cx.simulate_keystrokes("escape");
    for width in [560., 768., 1194., 1366.] {
        cx.simulate_resize(size(px(width), px(834.)));
        draw(cx);
        view.update(cx, |view, _| view.settle_tabs());
        draw(cx);
        let tab = cx.debug_bounds("tab-a").unwrap();
        let close = cx.debug_bounds("close-tab-a").unwrap();
        assert_eq!(tab.size.height, px(44.), "width={width}");
        assert_eq!(close.size, size(px(44.), px(44.)), "width={width}");
        assert!(close.top() >= tab.top() && close.bottom() <= tab.bottom());
        assert!(close.left() >= tab.left() && close.right() <= tab.right());
        if let Some(hint) = cx.debug_bounds("tab-shortcut-a") {
            assert!(hint.top() >= tab.top() && hint.bottom() <= tab.bottom());
            assert!(
                hint.right() <= close.left(),
                "hint and close overlap at {width}"
            );
        }
        for selector in ["new-tab", "split-editor", "tab-actions"] {
            let control = cx.debug_bounds(selector).unwrap();
            assert_eq!(
                control.size,
                size(px(44.), px(44.)),
                "{selector} at {width}"
            );
        }
    }
}
