//! Drawing and geometry shared by the desktop rail marks.
use super::*;

/// The columns a workspace badge and an agent row share. `icon` is set when
/// several hosts are listed: the agent row then has a host letter, and
/// workspace rows keep a slot of the same width so the status dots stay in
/// one column. A muted workspace draws its mark in that slot, so the rail
/// keeps it for one host too while one is muted.
#[derive(Clone, Copy)]
pub(super) struct Columns {
    pub(super) badge: f32,
    pub(super) dot: f32,
    pub(super) gap: f32,
    pub(super) height: f32,
    pub(super) icon: Option<f32>,
}

impl Columns {
    fn width(self) -> f32 {
        let gaps = self.gap * if self.icon.is_some() { 2. } else { 1. };
        self.badge + self.icon.unwrap_or(0.) + gaps + self.dot
    }
}

pub(super) fn rail_columns(
    font: &FontConfig,
    indicators: Indicators,
    multi: bool,
    slot: bool,
) -> Columns {
    let glyph = glyph_width(font);
    let icon = AGENT_ICON.min(font.size + 1.);
    Columns {
        badge: (glyph * 2.).max(if multi { glyph } else { icon }),
        dot: indicators.width(font),
        gap: MARK_GAP,
        height: line_height(font).max(icon),
        icon: slot.then_some(icon),
    }
}

/// The widest the column grid may be: the rail less an outline's pixel on
/// each side.
const RAIL_GRID: f32 = RAIL_WIDTH - 2.;

/// The smallest font the rail steps down to.
const RAIL_FONT_MIN: f32 = 8.;

/// `font`, stepped down in half pixels until the column grid fits the rail.
/// A font that already fits is returned as is.
pub(super) fn rail_font(
    font: &FontConfig,
    indicators: Indicators,
    multi: bool,
    slot: bool,
) -> FontConfig {
    let mut fitted = font.clone();
    while rail_columns(&fitted, indicators, multi, slot).width() > RAIL_GRID
        && fitted.size > RAIL_FONT_MIN
    {
        fitted.size = (fitted.size - 0.5).max(RAIL_FONT_MIN);
    }
    fitted
}

/// How much room the highlight can give the grid on every side and still sit
/// inside the rail and the row. Outline borders take a pixel of that room.
fn rail_pad(look: SidebarLook, row_h: f32, columns: Columns) -> f32 {
    let border = match look.density.highlight {
        Highlight::Outline => 1.,
        Highlight::Fill => 0.,
    };
    let room_x = (RAIL_WIDTH - columns.width()) / 2. - border;
    let room_y = (row_h - columns.height) / 2. - border;
    RAIL_PAD.min(room_x).min(room_y).max(0.)
}

/// A workspace or agent row whose highlight wraps the shared column grid
/// with the same padding on every side. The grid is centered in the rail, so
/// every such row shares one left edge and one width.
pub(super) fn aligned_row(
    key: &str,
    state: RowState,
    font: &FontConfig,
    look: SidebarLook,
    theme: &Theme,
    columns: Columns,
    marks: Div,
) -> Div {
    let row_h = look.row_height(line_height(font).max(HOST_MARK));
    let pad = rail_pad(look, row_h, columns);
    look.hover_group(
        div()
            .relative()
            .flex_none()
            .h(px(row_h))
            .w_full()
            .row()
            .items_center()
            .justify_center()
            .overflow_hidden(),
    )
    .child(
        look.paint_highlight(div().flex_none().p(px(pad)), key, state, theme)
            .child(marks),
    )
}

/// The badge column: a workspace number or, when several hosts are listed,
/// the agent's host letter. Fixed width, so both kinds of row share its left edge.
pub(super) fn mark_slot(selector: String, width: f32, height: f32, color: u32, text: &str) -> Div {
    div()
        .debug_selector(move || selector)
        .flex_none()
        .w(px(width))
        .h(px(height))
        .row()
        .items_center()
        .text_color(rgb(color))
        .child(label_text(text))
}

pub(super) fn agent_icon(selector: String, size: f32, color: u32, path: &'static str) -> Div {
    div()
        .debug_selector(move || selector)
        .flex_none()
        .size(px(size))
        .row()
        .items_center()
        .justify_center()
        .child(crate::icons::icon(path, size, color))
}

pub(super) fn dot_slot(selector: String, width: f32, height: f32, mark: Div) -> Div {
    div()
        .debug_selector(move || selector)
        .flex_none()
        .w(px(width))
        .h(px(height))
        .row()
        .items_center()
        .justify_center()
        .child(mark)
}

/// The status dot beside the header's bottom-right corner. A sidebar-colored
/// ring, then the dot, hangs outside the box and within its height, so the
/// stroke does not cross the dot and a compact row does not clip it. A rail
/// too narrow for that gap keeps the ring inside the box instead. The same
/// place is used for the accented host and the dimmed ones.
pub(super) fn host_status(selector: String, color: u32, ring: u32, rail_width: f32) -> Div {
    let ring_selector = selector.replacen("rail-host-status-", "rail-host-ring-", 1);
    let ring_size = HOST_DOT + 2. * HOST_DOT_RING;
    div()
        .debug_selector(move || ring_selector)
        .absolute()
        .inset_e(px(host_status_right(rail_width, ring_size)))
        .bottom(px(HOST_BORDER))
        .size(px(ring_size))
        .row()
        .items_center()
        .justify_center()
        .rounded_full()
        .bg(rgb(ring))
        .child(
            div()
                .debug_selector(move || selector)
                .size(px(HOST_DOT))
                .flex_none()
                .rounded_full()
                .bg(rgb(color)),
        )
}

/// `right` inset of the status ring. Negative when the ring hangs past the
/// box. Absolute `right` is the distance from the padding edge, and the
/// border is subtracted again: `left = mark - border - size - right`.
fn host_status_right(rail_width: f32, ring_size: f32) -> f32 {
    let content = (rail_width - HOST_BORDER).max(HOST_MARK);
    let side = (content - HOST_MARK) / 2.;
    if side + 0.5 >= HOST_DOT_GAP + ring_size {
        return -(HOST_BORDER + ring_size + HOST_DOT_GAP);
    }
    // The column cannot hold the box and a dot beside it. Sit inside.
    HOST_BORDER
}

/// A host header: the letter box, centered on the same line as the row grid
/// below it. Smaller than the selection highlight, and painted with the
/// workspace badge's chrome rather than a second copy of that highlight.
pub(super) fn host_header(font: &FontConfig, look: SidebarLook) -> Div {
    div()
        .relative()
        .flex_none()
        .h(px(look.row_height(line_height(font).max(HOST_MARK))))
        .w_full()
        .row()
        .items_center()
        .justify_center()
        .overflow_hidden()
}

/// Paint for one host header. The box is [`SidebarLook::badge_chrome`]: the
/// selected workspace's badge when this host holds that workspace, and the
/// same washes one step quieter otherwise. The letter stays the foreground
/// on the active host and the muted token on the others.
pub(super) struct HostChrome {
    pub(super) border: Rgba,
    pub(super) fill: Rgba,
    pub(super) letter: Rgba,
    pub(super) radius: f32,
}

pub(super) fn host_chrome(active: bool, look: SidebarLook, theme: &Theme) -> HostChrome {
    let badge = look.badge_chrome(active, theme);
    HostChrome {
        border: badge.border,
        fill: badge.fill,
        letter: rgb(if active {
            theme.foreground
        } else {
            theme.muted
        }),
        radius: badge.radius,
    }
}

/// A rail row: centered marks over the active layout's highlight, so the rail
/// keeps the expanded rows' look in every sidebar layout.
fn rail_row(
    key: &str,
    state: RowState,
    font: &FontConfig,
    look: SidebarLook,
    theme: &Theme,
) -> Div {
    look.mark(
        div()
            .relative()
            .flex_none()
            .h(px(look.row_height(line_height(font).max(HOST_MARK))))
            .row()
            .items_center()
            .justify_center()
            .overflow_hidden(),
        key,
        (state, HostMark::default()),
        theme,
    )
}

pub(super) fn rail_button(
    id: &'static str,
    font: &FontConfig,
    look: SidebarLook,
    theme: &Theme,
) -> Stateful<Div> {
    rail_row(id, RowState::default(), font, look, theme)
        .id(id)
        .debug_selector(move || id.into())
        .cursor_pointer()
}

/// The indicator centered on its line, whichever style draws it. A drawn
/// icon a little taller than the line spills evenly into the row's padding.
pub(super) fn indicator(status: AgentStatus, font: &FontConfig, indicators: Indicators) -> Div {
    div()
        .flex_none()
        .h(px(line_height(font)))
        .row()
        .items_center()
        .child(status_indicator(status, font, indicators))
}
