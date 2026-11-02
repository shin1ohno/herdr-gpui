use super::{
    overlay::{attach_with, open, poll},
    selection::acknowledge_size,
    *,
};
use crate::config::ChatLayoutMode;
use gpui::{Modifiers, TestAppContext, px, rgb, size};
use herdr_chat_protocol::Command;
use herdr_client::ConnectTarget;

#[gpui::test]
fn ipad_composer_keeps_44_point_buttons_inside_its_22_point_frame(cx: &mut TestAppContext) {
    let _ipad = crate::ipad_ui::ipad_layout();
    let (view, _peer, cx) = open(cx, ConnectTarget::Local);
    view.update(cx, |view, _| {
        view.config.chat.layout = ChatLayoutMode::Bubbles
    });
    let program = Program::new();
    let mut stream = attach_with(
        &view,
        &program,
        snapshot(cursor(1, 0), vec![agent(1, "Ready.")]),
        cx,
    );
    for width in [1194., 650.] {
        cx.simulate_resize(size(px(width), px(834.)));
        cx.run_until_parked();
        acknowledge_size(&view, cx);
        poll(&view, &program, cx);
        let frame = cx.debug_bounds("chat-composer-frame").unwrap();
        for selector in ["chat-attach", "chat-send"] {
            let button = cx.debug_bounds(selector).unwrap();
            assert_eq!(button.size, size(px(44.), px(44.)));
            assert!(button.left() >= frame.left() && button.right() <= frame.right());
            assert!(button.top() >= frame.top() && button.bottom() <= frame.bottom());
        }
        cx.update(|window, _| {
            let scale = window.scale_factor();
            assert!(
                window
                    .painted_quads()
                    .iter()
                    .any(|quad| quad.bounds == frame.scale(scale)
                        && (quad.corner_radii.top_left.0 - 22. * scale).abs() < 0.01)
            );
        });
        let disabled = cx.debug_bounds("chat-send").unwrap();
        cx.simulate_click(disabled.center(), Modifiers::default());
    }
    cx.simulate_input("ship it");
    poll(&view, &program, cx);
    let send = cx.debug_bounds("chat-send").unwrap();
    cx.update(|window, cx| {
        assert!(
            window.painted_quads().iter().any(|quad| quad.bounds
                == send.scale(window.scale_factor())
                && quad.background.as_solid() == Some(rgb(view.read(cx).theme.primary()).into()))
        );
    });
    cx.simulate_click(send.center(), Modifiers::default());
    assert!(
        matches!(read(&mut stream), Some(ClientMessage::Command {command: Command::Prompt {text, ..}, ..}) if text == "ship it")
    );
}

#[gpui::test]
fn ipad_stop_button_keeps_the_touch_size_and_cancel_action(cx: &mut TestAppContext) {
    let _ipad = crate::ipad_ui::ipad_layout();
    let (view, _peer, cx) = open(cx, ConnectTarget::Local);
    view.update(cx, |view, _| {
        view.config.chat.layout = ChatLayoutMode::Bubbles
    });
    let mut transcript = Transcript::new("Claude");
    transcript.status = Status::Working;
    let program = Program::new();
    let mut stream = attach_with(
        &view,
        &program,
        ServerMessage::Snapshot {
            cursor: cursor(1, 0),
            transcript: Box::new(transcript),
        },
        cx,
    );
    poll(&view, &program, cx);
    let stop = cx.debug_bounds("chat-stop").unwrap();
    assert_eq!(stop.size, size(px(44.), px(44.)));
    cx.simulate_click(stop.center(), Modifiers::default());
    assert!(matches!(
        read(&mut stream),
        Some(ClientMessage::Command {
            command: Command::Cancel,
            ..
        })
    ));
}
