use super::*;
use crate::sidebar::layout_tests::{fixture_window, full_draw};
use gpui::{px, size};

#[gpui::test]
fn ipad_activity_bar_keeps_working_panels_and_desktop_keeps_all(cx: &mut gpui::TestAppContext) {
    let (view, cx) = cx.add_window_view(fixture_window);
    cx.simulate_resize(size(px(1200.), px(1200.)));
    let excluded = [
        // keep-sorted start
        ActivityPanel::Automations,
        ActivityPanel::Issues,
        ActivityPanel::PullRequests,
        ActivityPanel::Tasks,
        ActivityPanel::Usage,
        // keep-sorted end
    ];
    for ipad in [false, true] {
        let _ipad = ipad.then(crate::ipad_ui::ipad_layout);
        view.update(cx, |view, cx| {
            view.config.sidebar_style.activity_bar = true;
            view.config.sidebar_style.automations = true;
            view.activity.tasks_listed = true;
            for &panel in ActivityPanel::ALL {
                assert_eq!(
                    view.activity_panel_listed(panel),
                    !ipad || !excluded.contains(&panel),
                    "{panel:?}"
                );
            }
            // A saved unsupported panel cannot strand the window on its warning.
            for panel in excluded {
                view.activity.panel = panel;
                assert_eq!(
                    view.activity_panel(),
                    if ipad {
                        ActivityPanel::Workspaces
                    } else {
                        panel
                    }
                );
            }
            view.activity.panel = ActivityPanel::Workspaces;
            cx.notify();
        });
        cx.update(|window, cx| {
            window.refresh();
            full_draw(window, cx).clear(cx);
        });
        for &panel in ActivityPanel::ALL {
            let selector = super::super::panel_selector(panel);
            assert_eq!(
                cx.debug_bounds(selector).is_some(),
                !ipad || !excluded.contains(&panel),
                "{selector}"
            );
        }
    }
}
