use super::{
    overlay::{attach_with, open, poll},
    *,
};
use crate::{
    config::ChatLayoutMode,
    i18n::{
        fl,
        leak_tests::{draw, uncut, under_en_xa},
    },
};
use gpui::Modifiers;
use herdr_chat_protocol::{Activity, ActivityKind, ActivityState, Turn, TurnEnd, TurnSummary};
use herdr_client::ConnectTarget;

#[test]
fn ipad_chat_omits_app_elapsed_time_in_status_working_turns_and_running_table() {
    under_en_xa("iPad chat without elapsed", |cx, recorder| {
        let _ipad = crate::ipad_ui::ipad_layout();
        let (view, _peer, cx) = open(cx, ConnectTarget::Local);
        view.update(cx, |view, _| {
            view.config.chat.layout = ChatLayoutMode::Bubbles
        });
        draw(cx);
        recorder.take();
        let mut transcript = Transcript::new("Claude");
        transcript.status = Status::Working;
        transcript.turn = Some(Turn {
            started_ms: now_ms().saturating_sub(12_000),
        });
        transcript.entries = vec![
            agent(1, "Working on it."),
            Entry {
                id: EntryId(2),
                kind: EntryKind::Turn(TurnSummary {
                    ended_ms: None,
                    duration_ms: 77_000,
                    end: TurnEnd::Done,
                    tokens: None,
                }),
            },
        ];
        transcript.activities.push(Activity {
            call_id: "shell".into(),
            detail: None,
            finished_ms: None,
            kind: ActivityKind::Shell,
            label: "build".into(),
            started_ms: now_ms().saturating_sub(65_000),
            state: ActivityState::Running,
        });
        assert_eq!(view::status_text(&transcript), fl!("chat-status-working"));
        let program = Program::new();
        let _stream = attach_with(
            &view,
            &program,
            ServerMessage::Snapshot {
                cursor: cursor(1, 0),
                transcript: Box::new(transcript),
            },
            cx,
        );
        poll(&view, &program, cx);
        let row = cx.debug_bounds("chat-running").unwrap();
        cx.simulate_click(row.center(), Modifiers::default());
        wait_until(|| {
            poll(&view, &program, cx);
            cx.debug_bounds("chat-running-row-0").is_some()
        });
        let lines = uncut(recorder.take());
        let count = fl!("chat-running-shells", count = 1);
        assert!(
            !lines
                .iter()
                .any(|line| line.chars().any(|ch| ch.is_ascii_digit()) && line != &count),
            "elapsed leaked: {lines:?}"
        );
        assert!(!lines.contains(&fl!("chat-running-time")));
    });
}
