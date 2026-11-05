use super::*;

fn draw(cx: &mut gpui::VisualTestContext) {
    cx.update(|window, cx| {
        window.refresh();
        window.draw(cx).clear(cx);
    });
}

#[gpui::test]
fn ipad_omits_the_account_control_and_slot_without_losing_tab_actions(cx: &mut TestAppContext) {
    let _ipad = crate::ipad_ui::ipad_layout();
    let (view, cx) = cx.add_window_view(crate::sidebar::layout_tests::fixture_window);
    for width in [560., 1194.] {
        cx.simulate_resize(size(px(width), px(834.)));
        for connected in [false, true] {
            view.update(cx, |view, cx| {
                view.menu.github = if connected {
                    crate::github::Auth::connected_fixture()
                } else {
                    crate::github::Auth::default()
                };
                cx.notify();
            });
            draw(cx);
            assert!(cx.debug_bounds("titlebar-avatar").is_none());
            assert!(cx.debug_bounds("titlebar-account-slot").is_none());
            let trailing = cx.debug_bounds("strip-titlebar-trailing").unwrap();
            let controls_width = if crate::menus::IN_WINDOW {
                px(0.)
            } else {
                cx.debug_bounds("window-controls")
                    .map_or(px(0.), |controls| controls.size.width)
            };
            assert_eq!(
                trailing.size.width, controls_width,
                "no empty account padding"
            );
            for selector in ["new-tab", "split-editor", "tab-actions"] {
                let control = cx.debug_bounds(selector).unwrap();
                assert_eq!(control.size, size(px(44.), px(44.)));
                assert!(control.right() <= trailing.left());
            }
        }
    }
}

#[gpui::test]
fn desktop_keeps_its_existing_account_slot(cx: &mut TestAppContext) {
    let _desktop = crate::ipad_ui::desktop();
    let (_, cx) = cx.add_window_view(header_window);
    cx.simulate_resize(size(px(1194.), px(834.)));
    draw(cx);
    assert!(cx.debug_bounds("titlebar-avatar").is_some());
    assert_eq!(
        cx.debug_bounds("titlebar-account-slot").unwrap().size.width,
        px(40.)
    );
}
