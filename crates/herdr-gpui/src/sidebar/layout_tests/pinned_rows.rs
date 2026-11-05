//! End padding and the pin must share a side even when long labels truncate.
use super::*;
use crate::{config::Density, preferences::pins::AgentPin};
use herdr_i18n::{Locale, Pseudo};

const LABEL: &str = "A renamed agent with a deliberately long task title that must truncate";

#[gpui::test]
fn pinned_rows_reserve_the_logical_end_at_narrow_widths(cx: &mut gpui::TestAppContext) {
    for (locale, rtl) in [(Locale::FALLBACK, false), (Pseudo::Bidi.locale(), true)] {
        crate::i18n::show(&locale);
        let (view, cx) = cx.add_window_view(|window, cx| {
            let mut view = fixture_window(window, cx);
            let mut listing = snapshot(1);
            listing.agents.truncate(1);
            listing.agents[0].name = Some(LABEL.into());
            listing.agents[0].display_agent = None;
            listing.panes = vec![ClientShellPane {
                pane_id: "p0".into(),
                workspace_id: "w0".into(),
                tab_id: "t0".into(),
                label: Some(LABEL.into()),
                cwd: None,
                foreground_cwd: None,
                focused: false,
                right_click_passthrough: false,
            }];
            view.live.snapshot = Some(Arc::new(listing));
            view.config.layout.mode = Density::Comfortable.into();
            view.config.usage.inline = false;
            view
        });
        cx.simulate_resize(size(px(545.), px(700.)));
        for width in [160., 200., 320.] {
            view.update(cx, |view, cx| {
                view.sidebar_width = Some(width);
                view.agent_pins = Default::default();
                cx.notify();
            });
            cx.update(|window, cx| full_draw(window, cx).clear(cx));
            let leading = cx.debug_bounds("mark-agent-p0").unwrap();
            view.update(cx, |view, cx| {
                let boot = &view.live.snapshot.as_ref().unwrap().boot_id;
                view.agent_pins.toggle(AgentPin::new("local", boot, "p0"));
                cx.notify();
            });
            cx.update(|window, cx| {
                cx.default_global::<TextProbes>().0.clear();
                full_draw(window, cx).clear(cx);
            });
            let row = cx
                .debug_bounds("agent-local-p0")
                .or_else(|| cx.debug_bounds("row-agent-p0"))
                .unwrap();
            let pin = cx.debug_bounds("agent-pin-local-p0").unwrap();
            let column = cx.debug_bounds("column-agent-p0").unwrap();
            assert_eq!(
                cx.debug_bounds("mark-agent-p0").unwrap(),
                leading,
                "rtl={rtl}, width={width}: pin moved the leading status"
            );
            assert!(
                pin.left() >= row.left() && pin.right() <= row.right(),
                "rtl={rtl}, width={width}: pin {pin:?} outside {row:?}"
            );
            eprintln!(
                "rtl={rtl}, width={width}: row={row:?}, column={column:?}, pin={pin:?}, leading={leading:?}"
            );
            if rtl {
                assert!(
                    column.left() >= pin.right(),
                    "rtl={rtl}, width={width}: column {column:?} overlaps pin {pin:?}"
                );
            } else {
                assert!(
                    column.right() <= pin.left(),
                    "rtl={rtl}, width={width}: column {column:?} overlaps pin {pin:?}"
                );
            }
            cx.update(|_, cx| {
                let (bounds, rendered, glyphs) = cx.global::<TextProbes>().0.get(LABEL).unwrap();
                assert!(!rendered.is_empty() && *glyphs <= bounds.size.width + px(1.));
            });
        }
    }
    crate::i18n::show(&Locale::FALLBACK);
}
