use super::{
    overlay::{attach_with, open, poll},
    timeline::conversation,
    *,
};
use crate::config::ChatLayoutMode;
use gpui::{TestAppContext, rgb};
use herdr_client::ConnectTarget;

#[gpui::test]
fn ipad_user_bubble_has_active_fill_and_a_small_bottom_right_corner(cx: &mut TestAppContext) {
    let _ipad = crate::ipad_ui::ipad_layout();
    let (view, _peer, cx) = open(cx, ConnectTarget::Local);
    view.update(cx, |view, _| {
        view.config.chat.layout = ChatLayoutMode::Bubbles
    });
    let program = Program::new();
    let _stream = attach_with(&view, &program, conversation(false), cx);
    poll(&view, &program, cx);
    let bubble = cx.debug_bounds("chat-bubble-1").unwrap();
    let column = cx.debug_bounds("chat-reading-column").unwrap();
    assert!(bubble.center().x > column.center().x);
    assert!(cx.debug_bounds("chat-bubble-2").is_none());
    cx.update(|window, cx| {
        let scale = window.scale_factor();
        let bounds = bubble.scale(scale);
        let fill = rgb(view.read(cx).theme.active).into();
        let quads = window.painted_quads();
        let quad = quads
            .iter()
            .find(|quad| quad.bounds == bounds && quad.background.as_solid() == Some(fill))
            .unwrap();
        for corner in [
            quad.corner_radii.top_left,
            quad.corner_radii.top_right,
            quad.corner_radii.bottom_left,
        ] {
            assert!((corner.0 - 18. * scale).abs() < 0.01);
        }
        assert!((quad.corner_radii.bottom_right.0 - 6. * scale).abs() < 0.01);
    });
}
