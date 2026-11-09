use super::{
    ipad_permission::{OTHER, TITLE, pending},
    overlay::{attach_with, open, poll},
    selection::acknowledge_size,
    *,
};
use crate::{
    config::ChatLayoutMode,
    i18n::{
        fl,
        leak_tests::{assert_translated, draw, uncut, under_en_xa},
    },
};
use gpui::{px, size};
use sheprd_chat_protocol::{PermissionKind, ToolKind};
use sheprd_client::ConnectTarget;

#[test]
fn ipad_permission_labels_and_composer_are_translated_at_wide_and_narrow_widths() {
    under_en_xa("iPad chat permission", |cx, recorder| {
        let _ipad = crate::ipad_ui::ipad_layout();
        let (view, _peer, cx) = open(cx, ConnectTarget::Local);
        view.update(cx, |view, _| {
            view.config.chat.layout = ChatLayoutMode::Bubbles
        });
        draw(cx);
        // A resize can reshape only a cut of chrome whose whole line was
        // cached on the first frame. Keep that measurement when removing cuts.
        let baseline = recorder.take();
        let program = Program::new();
        let _stream = attach_with(
            &view,
            &program,
            pending(ToolKind::Execute, Some("command")),
            cx,
        );
        let mut seen = recorder.take();
        for width in [1194., 650.] {
            cx.simulate_resize(size(px(width), px(834.)));
            cx.run_until_parked();
            acknowledge_size(&view, cx);
            poll(&view, &program, cx);
            seen.extend(recorder.take());
            let measured = uncut(baseline.iter().chain(&seen).cloned().collect());
            let labels: Vec<_> = seen
                .iter()
                .filter(|line| measured.contains(line))
                .cloned()
                .collect();
            assert_translated(
                "iPad permission and composer",
                &labels,
                &["Claude".into(), TITLE.into(), OTHER.into()],
            );
            for label in [
                fl!("chat-permission-run-command"),
                fl!("chat-permission-allow"),
                fl!("chat-permission-always-allow"),
                fl!("chat-permission-deny"),
            ] {
                assert!(
                    labels.contains(&label),
                    "missing label {label:?} in {labels:?}"
                );
            }
            assert!(labels.iter().any(|line| line == OTHER));
            assert!(!labels.iter().any(|line| line.starts_with("Agent option")));
            let card = cx.debug_bounds("chat-permission").unwrap();
            for selector in [
                "chat-answer-0",
                "chat-answer-1",
                "chat-answer-2",
                "chat-answer-3",
                "chat-answer-4",
            ] {
                let button = cx.debug_bounds(selector).unwrap();
                assert!(button.left() >= card.left() && button.right() <= card.right());
                assert!(button.size.height >= px(48.));
            }
        }
    });
}

#[test]
fn ipad_permission_fallback_titles_and_other_options_are_sanitized() {
    for kind in [ToolKind::Edit, ToolKind::Execute] {
        under_en_xa("iPad permission data", |cx, recorder| {
            let _ipad = crate::ipad_ui::ipad_layout();
            let (view, _peer, cx) = open(cx, ConnectTarget::Local);
            draw(cx);
            recorder.take();
            let mut message = pending(kind, Some("command"));
            if let ServerMessage::Snapshot { transcript, .. } = &mut message {
                transcript.permissions[0].title = "Edit\u{202e} config".into();
                transcript.permissions[0]
                    .options
                    .retain(|option| option.kind == PermissionKind::Other);
                transcript.permissions[0].options[0].name = "Review\u{202e} first".into();
            }
            let program = Program::new();
            let _stream = attach_with(&view, &program, message, cx);
            poll(&view, &program, cx);
            let lines = uncut(recorder.take());
            assert!(lines.iter().any(|line| line == "Edit config"), "{lines:?}");
            assert!(lines.iter().any(|line| line == "Review first"), "{lines:?}");
            assert!(!lines.iter().any(|line| line.contains('\u{202e}')));
        });
    }
}
