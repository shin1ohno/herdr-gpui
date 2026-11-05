use super::*;
use gpui::{Bounds, Pixels, point};

fn disjoint(a: Bounds<Pixels>, b: Bounds<Pixels>) -> bool {
    a.right() <= b.left() || b.right() <= a.left() || a.bottom() <= b.top() || b.bottom() <= a.top()
}

/// Exercise the real app paths, not a synthetic header supplied a clearance.
/// The compact-only tests missed the omitted wide-window integration.
#[gpui::test]
fn wide_headers_clear_native_controls_without_moving_the_body(cx: &mut TestAppContext) {
    let _ipad = crate::ipad_ui::ipad_layout();
    for locale in ["en-US", "en-XB"] {
        crate::i18n::show(&locale.parse().unwrap());
        for (position, collapsed) in [
            ("top", "compact"),
            ("top", "hidden"),
            ("bottom", "compact"),
            ("bottom", "hidden"),
        ] {
            let (view, cx) = cx.add_window_view(fixture_window);
            view.update(cx, |view, _| {
                view.settings.shared = Some(
                    crate::herdr_settings::Settings::parse_text(&format!(
                        "[ui]\ntab_bar_position = '{position}'\nsidebar_collapsed_mode = '{collapsed}'"
                    ))
                    .unwrap(),
                );
            });
            for width in [560., 834., 1200.] {
                cx.simulate_resize(size(px(width), px(834.)));
                let mut baseline = [None, None];
                for inset in [66., 96., 0., 66.] {
                    let _controls = simulate_window_controls(Edges {
                        left: px(inset),
                        ..Default::default()
                    });
                    for (state, visible) in [true, false].into_iter().enumerate() {
                        view.update(cx, |view, _| {
                            view.sidebar_visible = visible;
                            view.sidebar_slide = Default::default();
                        });
                        draw(cx);
                        // The navigation-fit canvas may request the next frame.
                        cx.run_until_parked();
                        draw(cx);
                        let toggle = cx.debug_bounds("toggle-sidebar").unwrap();
                        let back = cx.debug_bounds("titlebar-back").unwrap();
                        let forward = cx.debug_bounds("titlebar-forward").unwrap();
                        let native = Bounds::new(point(px(0.), px(0.)), size(px(inset), px(44.)));
                        let controls = [toggle, back, forward];
                        for control in controls {
                            assert!(
                                control.left() >= px(inset),
                                "{locale} {position} {width} inset={inset} visible={visible}: {control:?}"
                            );
                            assert!(control.right() <= px(width));
                            assert_eq!(control.size, size(px(44.), px(44.)));
                            assert!(disjoint(control, native));
                            assert_eq!(control.top(), px(0.));
                        }
                        for (i, control) in controls.into_iter().enumerate() {
                            for other in &controls[i + 1..] {
                                assert!(disjoint(control, *other));
                            }
                        }
                        let body = cx.debug_bounds("window-body").unwrap();
                        let terminal = cx.debug_bounds("terminal").unwrap();
                        let geometry = (
                            body.top(),
                            body.size.height,
                            terminal.top(),
                            terminal.size.height,
                        );
                        if let Some(expected) = baseline[state] {
                            assert_eq!(geometry, expected);
                        } else {
                            baseline[state] = Some(geometry);
                        }
                        let merged = position == "top" && locale == "en-US";
                        assert_eq!(body.top(), px(if merged { 0. } else { 44. }));
                        let header = cx
                            .debug_bounds(if merged && visible {
                                "sidebar-titlebar"
                            } else if merged {
                                "strip-titlebar-leading"
                            } else {
                                "titlebar"
                            })
                            .unwrap();
                        assert_eq!(header.top(), px(0.));
                        assert_eq!(header.size.height, px(44.));
                        cx.simulate_click(toggle.center(), Modifiers::default());
                        cx.run_until_parked();
                        assert_eq!(view.read_with(cx, |view, _| view.sidebar_visible), !visible);
                    }
                }
            }
        }
    }
}

#[gpui::test]
fn wide_navigation_tracks_changes_to_the_physical_corner(cx: &mut TestAppContext) {
    let _ipad = crate::ipad_ui::ipad_layout();
    crate::i18n::show(&"en-US".parse().unwrap());
    let (view, cx) = cx.add_window_view(fixture_window);
    view.update(cx, |view, _| {
        view.settings.shared = Some(
            crate::herdr_settings::Settings::parse_text(
                "[ui]\ntab_bar_position = 'top'\nsidebar_collapsed_mode = 'compact'",
            )
            .unwrap(),
        );
    });
    cx.simulate_resize(size(px(560.), px(834.)));
    // A macOS host starts with traffic-light clearance. The second
    // inset must exceed it to detect a stale constant in the actual view.
    for inset in [66., 96., 0., 66.] {
        let _controls = simulate_window_controls(Edges {
            left: px(inset),
            ..Default::default()
        });
        draw(cx);
        cx.run_until_parked();
        draw(cx);
        for name in ["toggle-sidebar", "titlebar-back", "titlebar-forward"] {
            let control = cx.debug_bounds(name).unwrap();
            assert!(
                control.left() >= px(inset),
                "{name} overlaps the physical corner: inset={inset}, bounds={control:?}"
            );
        }
    }
}
