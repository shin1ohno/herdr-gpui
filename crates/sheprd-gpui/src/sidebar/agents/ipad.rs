use super::*;
/// The inner mark, independent of row spacing and the configured icon set.
pub(crate) fn status_dot(status: AgentStatus, diameter: f32, indicators: Indicators) -> Div {
    draw_dot(
        status,
        diameter,
        indicators.color(status),
        false,
        indicators.fade(),
    )
}

fn draw_dot(status: AgentStatus, diameter: f32, color: u32, bold: bool, fade: f32) -> Div {
    if status == AgentStatus::Working {
        return div().size(px(diameter)).child(icon_mark(
            StatusIcons::Dots,
            status,
            diameter,
            color,
            fade,
        ));
    }
    let size = if status == AgentStatus::Unknown && !bold {
        diameter * STATUS_DOT_UNKNOWN / STATUS_WIDTH
    } else {
        diameter
    };
    div()
        .size(px(size))
        .rounded_full()
        .border_1()
        .when(bold, |dot| dot.border_2())
        .border_color(rgb(color))
        .when(status != AgentStatus::Idle, |dot| dot.bg(rgb(color)))
}
