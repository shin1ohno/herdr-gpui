use super::{
    overlay::{attach_with, open, poll},
    *,
};
use crate::config::Theme;
use gpui::{Modifiers, Styled, TestAppContext, px, rgb};
use herdr_chat_protocol::{
    Command, Permission, PermissionId, PermissionKind, PermissionOption, Tool, ToolKind, ToolStatus,
};
use herdr_client::ConnectTarget;

pub(super) const TITLE: &str = "cargo test";
pub(super) const OTHER: &str = "Ask the administrator";

pub(super) fn pending(kind: ToolKind, call_id: Option<&str>) -> ServerMessage {
    let mut transcript = Transcript::new("Claude");
    transcript.status = Status::Working;
    transcript.entries = vec![Entry {
        id: EntryId(1),
        kind: EntryKind::Tool(Tool {
            call_id: "command".into(),
            content: Vec::new(),
            kind,
            locations: Vec::new(),
            parent: None,
            status: ToolStatus::Pending,
            title: TITLE.into(),
        }),
    }];
    transcript.permissions = vec![Permission {
        id: PermissionId(7),
        call_id: call_id.map(str::to_owned),
        title: TITLE.into(),
        options: [
            PermissionKind::AllowOnce,
            PermissionKind::AllowAlways,
            PermissionKind::RejectOnce,
            PermissionKind::RejectAlways,
            PermissionKind::Other,
        ]
        .into_iter()
        .enumerate()
        .map(|(index, kind)| PermissionOption {
            id: format!("choice-{index}"),
            kind,
            name: if kind == PermissionKind::Other {
                OTHER.into()
            } else {
                format!("Agent option {index}")
            },
        })
        .collect(),
    }];
    ServerMessage::Snapshot {
        cursor: cursor(1, 0),
        transcript: Box::new(transcript),
    }
}

#[gpui::test]
fn ipad_permission_buttons_are_48_points_and_answer_with_the_original_ids(cx: &mut TestAppContext) {
    let _ipad = crate::ipad_ui::ipad_layout();
    let (view, _peer, cx) = open(cx, ConnectTarget::Local);
    view.update(cx, |view, _| view.theme = Theme::builtin("Nord").unwrap());
    let program = Program::new();
    let mut stream = attach_with(
        &view,
        &program,
        pending(ToolKind::Execute, Some("command")),
        cx,
    );
    poll(&view, &program, cx);
    assert!(cx.debug_bounds("chat-permission-command").is_some());
    let card = cx.debug_bounds("chat-permission").unwrap();
    for selector in [
        "chat-answer-0",
        "chat-answer-1",
        "chat-answer-2",
        "chat-answer-3",
        "chat-answer-4",
    ] {
        let bounds = cx.debug_bounds(selector).unwrap();
        assert_eq!(bounds.size.height, px(48.));
        assert!(bounds.left() >= card.left() && bounds.right() <= card.right());
    }
    let allow = cx.debug_bounds("chat-answer-0").unwrap();
    cx.update(|window, cx| {
        let theme = &view.read(cx).theme;
        let quads = window.painted_quads();
        assert!(
            quads
                .iter()
                .any(|quad| quad.bounds == allow.scale(window.scale_factor())
                    && quad.background.as_solid()
                        == Some(rgb(crate::ipad_ui::accent(theme)).into()))
        );
        assert!(
            quads
                .iter()
                .any(|quad| quad.bounds == card.scale(window.scale_factor())
                    && quad.border_color == rgb(theme.ink(theme.palette[3])).into())
        );
    });
    view.read_with(cx, |view, _| {
        let mut line = view.chat_permission_command(TITLE);
        assert_eq!(
            line.text_style().font_family.as_deref(),
            Some(view.config.terminal.family.as_str())
        );
    });
    for (index, selector) in [
        "chat-answer-0",
        "chat-answer-1",
        "chat-answer-2",
        "chat-answer-3",
        "chat-answer-4",
    ]
    .into_iter()
    .enumerate()
    {
        let bounds = cx.debug_bounds(selector).unwrap();
        cx.simulate_click(bounds.center(), Modifiers::default());
        assert!(
            matches!(read(&mut stream), Some(ClientMessage::Command { command: Command::Answer { permission: PermissionId(7), option }, .. }) if option == format!("choice-{index}"))
        );
    }
}

#[gpui::test]
fn only_a_matching_execute_call_gets_a_command_line(cx: &mut TestAppContext) {
    let _ipad = crate::ipad_ui::ipad_layout();
    for (kind, call_id) in [
        (ToolKind::Edit, Some("command")),
        (ToolKind::Execute, Some("missing")),
        (ToolKind::Execute, None),
    ] {
        let (view, _peer, cx) = open(cx, ConnectTarget::Local);
        let program = Program::new();
        let _stream = attach_with(&view, &program, pending(kind, call_id), cx);
        poll(&view, &program, cx);
        assert!(cx.debug_bounds("chat-permission-heading").is_some());
        assert!(cx.debug_bounds("chat-permission-command").is_none());
    }
}

#[gpui::test]
fn ipad_long_other_option_wraps_inside_a_narrow_permission_card(cx: &mut TestAppContext) {
    let _ipad = crate::ipad_ui::ipad_layout();
    let (view, _peer, cx) = open(cx, ConnectTarget::Local);
    cx.simulate_resize(gpui::size(px(650.), px(834.)));
    cx.run_until_parked();
    let mut message = pending(ToolKind::Execute, Some("command"));
    if let ServerMessage::Snapshot { transcript, .. } = &mut message {
        transcript.permissions[0]
            .options
            .retain(|option| option.kind == PermissionKind::Other);
        transcript.permissions[0].options[0].name =
            "Approve this operation after checking with the administrator".into();
    }
    let program = Program::new();
    let _stream = attach_with(&view, &program, message, cx);
    selection::acknowledge_size(&view, cx);
    poll(&view, &program, cx);
    let card = cx.debug_bounds("chat-permission").unwrap();
    let button = cx.debug_bounds("chat-answer-0").unwrap();
    assert!(button.left() >= card.left() && button.right() <= card.right());
    assert!(button.size.height > px(48.));
}
