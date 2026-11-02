//! Touch-sized rows with bounded columns. The same widths clip labels and
//! place the count and badge, so long peer text cannot move trailing content.

use crate::{
    direction::Logical,
    i18n::fl,
    ipad_ui::{
        accent, harness,
        metrics::{DOT, TOUCH},
    },
    sidebar::{
        agents::status_dot,
        cell::{
            AgentRow, RowContext, RowLayout, RowState, WorkspaceRow,
            ipad_meta::{AgentMeta, SpaceMeta},
        },
        metrics::segment_budgets,
        row::{RowLift, RowTree},
    },
};
use gpui::{prelude::*, *};

const SPACE_ROW: f32 = 56.;
const AGENT_ROW: f32 = 44.;
const SPACE_ICON: f32 = 13.;
const AGENT_ICON: f32 = 18.;
const CHILD_INDENT: f32 = 22.;
const GAP: f32 = 6.;

pub(in crate::sidebar) struct Ipad;

#[derive(Debug)]
pub(in crate::sidebar) struct SpaceWidths {
    pub(in crate::sidebar) name: f32,
    pub(in crate::sidebar) badge: Option<f32>,
    pub(in crate::sidebar) count: Option<f32>,
}

fn label(key: String, text: String, width: f32) -> Div {
    // i18n: data
    div()
        .debug_selector(move || key.clone())
        .w(px(width.max(0.)))
        .flex_none()
        .overflow_hidden()
        .truncate()
        .child(text)
}

fn shell(key: &str, height: f32, indent: f32, state: RowState, cx: &RowContext<'_>) -> Div {
    let theme = cx.theme;
    div()
        .debug_selector(|| format!("row-{key}"))
        .relative()
        .w_full()
        .h(px(height))
        .flex_none()
        .overflow_hidden()
        .ps(px(cx.look.content_x() + indent))
        .pe(px(cx.look.content_x()))
        .cursor_pointer()
        .map(|row| {
            if !state.selected {
                return cx.look.mark(row, key, (state, cx.mark), theme);
            }
            let layer = div()
                .debug_selector(|| format!("ipad-selection-{key}"))
                .absolute()
                .inset_0()
                .border_s(px(2.))
                .border_color(rgb(accent(theme)))
                .bg(Hsla::from(rgb(accent(theme))).opacity(0.15));
            let row = row.when(state.lift == RowLift::Lifted, |row| {
                row.bg(rgb(theme.sidebar_background()))
            });
            cx.look.washed(row, key, cx.mark, layer)
        })
}

impl Ipad {
    /// The name, badge, and count arguments are natural widths. Reserve a name
    /// floor for identification, then favor attention badges over tab counts.
    pub(in crate::sidebar) fn space_widths(
        width: f32,
        name: f32,
        badge: Option<f32>,
        count: f32,
    ) -> SpaceWidths {
        let mut remaining = (width - DOT - GAP).max(0.);
        let floor = name.min(remaining / 2.);
        let badge = badge.filter(|badge| *badge + GAP <= remaining - floor);
        if let Some(badge) = badge {
            remaining -= badge + GAP;
        }
        let count = (count + GAP <= remaining - floor).then_some(count);
        if let Some(count) = count {
            remaining -= count + GAP;
        }
        SpaceWidths {
            name: remaining,
            badge,
            count,
        }
    }

    fn indent(tree: RowTree, cx: &RowContext<'_>) -> f32 {
        cx.nest
            + if tree == RowTree::None {
                0.
            } else {
                CHILD_INDENT
            }
    }

    pub(in crate::sidebar) fn label_start(tree: RowTree, cx: &RowContext<'_>) -> f32 {
        cx.look.content_x() + Self::indent(tree, cx) + DOT + GAP
    }

    pub(in crate::sidebar) fn space(
        row: WorkspaceRow<'_>,
        meta: Option<SpaceMeta>,
        state: RowState,
        cx: &RowContext<'_>,
    ) -> Div {
        let theme = cx.theme;
        let scale = cx.font.size / 12.;
        let indent = Self::indent(row.tree, cx);
        let width = (cx.look.content_width(cx.width) - indent).max(0.);
        let meta = meta.unwrap_or_else(|| SpaceMeta {
            tab_count: 0,
            needs_you: row.status == herdr_client::protocol::AgentStatus::Blocked,
            lead: None,
            branch: row.branch().map(str::to_owned),
            worktree: row
                .workspace
                .worktree
                .as_ref()
                .is_some_and(|tree| tree.is_linked_worktree),
            reason: None,
        });
        let count = fl!("ipad-sidebar-tabs", count = meta.tab_count);
        let badge = meta.needs_you.then(|| fl!("ipad-sidebar-needs-you"));
        let small = 11. * scale;
        let widths = Self::space_widths(
            width,
            row.label.chars().count() as f32 * 14. * scale * 0.62,
            badge
                .as_ref()
                .map(|text| text.chars().count() as f32 * small * 0.62 + 10.),
            count.chars().count() as f32 * small * 0.62 + 2.,
        );
        let id = &row.workspace.workspace_id;
        let first = div()
            .row()
            .items_center()
            .gap(px(GAP))
            .h(px(22.))
            .w(px(width))
            .child(div().size(px(DOT)).flex_none().child(status_dot(
                row.status,
                DOT,
                cx.status_indicators(),
            )))
            .child(
                label(
                    format!("ipad-space-name-{id}"),
                    row.label.to_owned(),
                    widths.name,
                )
                .text_size(px(14. * scale))
                .font_weight(if state.selected {
                    FontWeight::BOLD
                } else {
                    FontWeight::NORMAL
                }),
            )
            .children(widths.count.map(|width| {
                label(format!("ipad-tab-count-{id}"), count, width)
                    .text_size(px(small))
                    .text_color(rgb(theme.subtext()))
            }))
            .children(badge.zip(widths.badge).map(|(text, width)| {
                label(format!("ipad-needs-you-{id}"), text, width)
                    .px(px(4.))
                    .text_size(px(small))
                    .font_weight(FontWeight::BOLD)
                    .rounded(px(4.))
                    .bg(rgb(theme.palette[3]))
                    .text_color(rgb(theme.text_on(theme.palette[3])))
            }));
        let branch = if meta.worktree {
            fl!("ipad-sidebar-worktree")
        } else {
            meta.branch.unwrap_or_default()
        };
        let lead = meta.lead;
        let name = lead.as_ref().map_or("", |lead| lead.name.as_str());
        let word = meta
            .reason
            .as_deref()
            .or_else(|| lead.as_ref().map(|lead| lead.state.as_str()))
            .unwrap_or("");
        let icon_room = if lead.is_some() { SPACE_ICON + 5. } else { 0. };
        let dot_room = DOT + GAP;
        let glyph = 13. * scale * 0.62;
        let branch_room = if branch.is_empty() {
            0.
        } else {
            2. * GAP + glyph
        };
        let available = (width - dot_room - icon_room - branch_room - GAP).max(0.);
        let budgets = segment_budgets(
            &[
                name.chars().count(),
                word.chars().count(),
                branch.chars().count(),
            ],
            (available / glyph).floor() as usize,
        );
        let detail = div()
            .debug_selector(|| format!("ipad-space-detail-{id}"))
            .ps(px(dot_room))
            .w(px(width))
            .h(px(20.))
            .row()
            .items_center()
            .gap(px(GAP))
            .text_size(px(13. * scale))
            .text_color(rgb(theme.subtext()))
            .children(lead.as_ref().map(|lead| {
                harness::row(lead.icon, SPACE_ICON, lead.name.clone().into(), theme)
                    .w(px(icon_room + budgets[0] as f32 * glyph))
                    .flex_none()
                    .overflow_hidden()
            }))
            .child(label(
                format!("ipad-space-state-{id}"),
                word.to_owned(),
                budgets[1] as f32 * glyph,
            ))
            .when(!branch.is_empty(), |detail| {
                detail
                    .child(div().w(px(glyph)).flex_none().child("·"))
                    .child(label(
                        format!("ipad-branch-{id}"),
                        branch,
                        budgets[2] as f32 * glyph,
                    ))
            });
        let mut result = shell(row.label, SPACE_ROW, indent, state, cx)
            .flex()
            .flex_col()
            .justify_center()
            .text_size(px(14. * scale))
            .line_height(px(18. * scale))
            .child(first)
            .child(detail);
        if let Some(fold) = row.fold {
            // The fold keeps its original click isolation, with a touch-sized
            // target on the leading edge rather than a tiny glyph hitbox.
            result = result.child(
                fold.element(theme)
                    .absolute()
                    .inset_s(px(0.))
                    .top(px(6.))
                    .size(px(TOUCH))
                    .flex()
                    .items_center()
                    .justify_start(),
            );
        }
        result
    }

    pub(in crate::sidebar) fn agent_row(
        row: AgentRow<'_>,
        meta: Option<AgentMeta>,
        state: RowState,
        cx: &RowContext<'_>,
    ) -> Div {
        let theme = cx.theme;
        let scale = cx.font.size / 12.;
        let width = cx.look.content_width(cx.width).max(0.);
        let meta = meta.unwrap_or_else(|| AgentMeta {
            text: row.place.map_or("", |(space, _)| space).to_owned(),
            needs_you: false,
        });
        let glyph = 12. * scale * 0.62;
        let available = (width - DOT - AGENT_ICON - 5. - 2. * GAP).max(0.);
        // Keep the name's natural width up to half the room after the dot and icon.
        let floor = (row.name.chars().count() as f32 * glyph).min(available / 2.);
        let natural = meta.text.chars().count() as f32 * glyph;
        let fitted = natural.min(available - floor);
        // Short metadata must fit whole; longer text needs at least two glyphs.
        let meta_width = if natural > 0. && fitted < natural.min(2. * glyph) {
            None
        } else {
            Some(fitted)
        };
        let name_width = meta_width.map_or(available + GAP, |meta| available - meta);
        shell(&row.key, AGENT_ROW, 0., state, cx)
            .row()
            .items_center()
            .gap(px(GAP))
            .child(div().size(px(DOT)).flex_none().child(status_dot(
                row.status,
                DOT,
                cx.status_indicators(),
            )))
            .child(
                harness::row(row.icon, AGENT_ICON, row.name.to_owned().into(), theme)
                    .debug_selector(|| format!("ipad-agent-name-{}", row.key))
                    .w(px(name_width + AGENT_ICON + 5.))
                    .flex_none()
                    .overflow_hidden(),
            )
            .children(meta_width.map(|width| {
                label(format!("ipad-agent-meta-{}", row.key), meta.text, width)
                    .text_size(px(12. * scale))
                    .font_weight(if meta.needs_you {
                        FontWeight::BOLD
                    } else {
                        FontWeight::NORMAL
                    })
                    .text_color(rgb(if meta.needs_you {
                        theme.ink(theme.palette[3])
                    } else {
                        theme.subtext()
                    }))
            }))
    }
}

impl RowLayout for Ipad {
    fn workspace(&self, row: WorkspaceRow<'_>, state: RowState, cx: &RowContext<'_>) -> Div {
        Self::space(row, None, state, cx)
    }
    fn agent(&self, row: AgentRow<'_>, state: RowState, cx: &RowContext<'_>) -> Div {
        Self::agent_row(row, None, state, cx)
    }
}
