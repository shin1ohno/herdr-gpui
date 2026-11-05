//! Touch tab cells. The strip measures the same marks and chips this draws.
use super::*;
use crate::ipad_ui::{
    kbd,
    metrics::{STRIP_BUTTON, TAB_STRIP},
};

const TAB_CLOSE: f32 = crate::ipad_ui::metrics::TOUCH;
const TAB_DOT: f32 = 8.;
const TAB_ICON: f32 = 16.;
const GAP: f32 = 8.;
const PAD: f32 = 8.;

pub(crate) struct TabHint {
    pub(crate) style: ShortcutStyle,
    pub(crate) text: SharedString,
    /// Shaped text plus the existing kbd chip's padding and borders.
    pub(crate) width: f32,
}

impl TabHint {
    pub(crate) fn occupied_width(&self) -> f32 {
        self.width + GAP
    }
}

/// Additional marks not represented by the desktop TabShape.
pub(crate) fn extra_width(mark: &Mark) -> f32 {
    match mark {
        Mark::Herdr {
            status, chat, hint, ..
        } => {
            let dot = if *chat && *status == AgentStatus::Working {
                TAB_DOT + GAP
            } else {
                0.
            };
            dot + hint.as_ref().map_or(0., TabHint::occupied_width)
        }
        Mark::Hinted { hint, .. } => hint.occupied_width(),
        _ => 0.,
    }
}

pub(super) struct Ipad;

impl TabLayout for Ipad {
    fn width(&self, title: f32, shape: TabShape, _: &FontConfig, _: Indicators) -> f32 {
        let lead = match shape.lead {
            Lead::Nothing => 0.,
            Lead::Status => TAB_DOT + GAP,
            Lead::Icon => TAB_ICON + GAP,
        };
        2. * PAD
            + 1.
            + title
            + GAP
            + TAB_CLOSE
            + lead
            + (usize::from(shape.zoomed) + usize::from(shape.muted)) as f32 * (GAP + ZOOM_ICON)
    }

    fn colors(&self, state: TabState, theme: &Theme) -> (u32, u32) {
        if state.selected {
            let fill = theme.primary_wash();
            (fill, crate::contrast::ink(theme.foreground, &[fill], 4.5))
        } else {
            (theme.surface, theme.subtext())
        }
    }

    fn tab(
        &self,
        base: Stateful<Div>,
        face: TabFace,
        state: TabState,
        cx: &TabContext<'_>,
    ) -> Stateful<Div> {
        let TabFace {
            label,
            mark,
            zoomed,
            muted,
            probe,
            close,
        } = face;
        let (mark, page_hint) = match mark {
            Mark::Hinted { mark, hint } => (*mark, Some(hint)),
            mark => (mark, None),
        };
        let (background, text) = self.colors(state, cx.theme);
        let mut tab = base
            .relative()
            .flex_none()
            .flex()
            .items_center()
            .h(px(TAB_STRIP))
            .px(px(PAD))
            .gap(px(GAP))
            .border_r_1()
            .border_color(rgb(cx.theme.active))
            .bg(cx.fill.over(cx.theme.surface, background))
            .text_color(rgb(text))
            .whitespace_nowrap()
            .cursor_pointer();
        match &mark {
            Mark::Herdr {
                status,
                agent,
                chat,
                ..
            } => {
                if *status == AgentStatus::Working {
                    tab = tab.child(
                        // The W0 dot is not re-exported by sidebar yet. Keep
                        // the specified size/color until that shared export lands.
                        div()
                            .size(px(TAB_DOT))
                            .flex_none()
                            .rounded_full()
                            .bg(rgb(cx.indicators.color(*status)))
                            .debug_selector(probe.status(cx.slot)),
                    );
                }
                if *chat && let Some(agent) = agent {
                    let selector = match &probe {
                        Probe::Herdr(id) => cx.slot.selector(&format!("tab-chat-icon-{id}")),
                        Probe::Page(id) => cx.slot.selector(&format!("tab-chat-icon-{id}")),
                    };
                    tab = tab.child(
                        div()
                            .debug_selector(move || selector.clone())
                            .size(px(TAB_ICON))
                            .flex_none()
                            .child(agent.element(TAB_ICON, text)),
                    );
                }
            }
            _ => tab = tab.children(lead_mark(&mark, &probe, TAB_ICON, text, cx)),
        }
        tab = tab
            .child(label)
            .children(zoom_mark(zoomed, &probe, text, cx))
            .children(mute_mark(muted, &probe, text, cx));
        let hint = match mark {
            Mark::Herdr { hint, .. } => hint,
            _ => page_hint,
        };
        if let Some(hint) = hint
            && cx.hardware
            && hint.style == cx.style
        {
            let selector = match &probe {
                Probe::Herdr(id) => cx.slot.selector(&format!("tab-shortcut-{id}")),
                Probe::Page(id) => cx.slot.selector(&format!("tab-shortcut-{id}")),
            };
            tab = tab.child(
                kbd(hint.text, cx.theme, cx.font)
                    .debug_selector(move || selector.clone())
                    .w(px(hint.width)),
            );
        }
        tab.child(close_button(close, &probe, TAB_CLOSE, text, cx))
            .when(state.selected, |tab| {
                tab.child(
                    div()
                        .absolute()
                        .left_0()
                        .right_0()
                        .bottom_0()
                        .h(px(2.))
                        .bg(rgb(selection_color(cx.theme))),
                )
            })
    }

    fn new_tab(&self, base: Stateful<Div>, icon: AnyElement, theme: &Theme) -> Stateful<Div> {
        base.size(px(STRIP_BUTTON))
            .flex_none()
            .flex()
            .items_center()
            .justify_center()
            .cursor_pointer()
            .hover(|style| style.bg(rgb(theme.active)))
            .child(icon)
    }

    fn new_tab_width(&self) -> f32 {
        STRIP_BUTTON
    }
    fn min_width(&self) -> f32 {
        0.
    }
    fn panes(&self) -> &'static dyn panes::PaneChrome {
        &IpadPanes
    }
}

/// iPad panes keep their cell area free of hover-dependent title overlays.
struct IpadPanes;

impl panes::PaneChrome for IpadPanes {
    fn gutter(&self) -> f32 {
        1.
    }
    fn veil(&self) -> u8 {
        0x50
    }
    fn pane(
        &self,
        _: panes::PaneFace,
        cells: AnyElement,
        _: &panes::PaneContext<'_>,
    ) -> AnyElement {
        cells
    }
}

/// Tab selection is separate from the iPad focus/pane accent (Nord's cyan).
/// Keep the underline legible against both the selected fill and strip.
fn selection_color(theme: &Theme) -> u32 {
    crate::contrast::ink(
        theme.primary(),
        &[theme.primary_wash(), theme.surface],
        theme.contrast.mark_ratio(),
    )
}

#[cfg(test)]
mod tests;
