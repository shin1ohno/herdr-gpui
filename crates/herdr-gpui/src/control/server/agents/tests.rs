#![allow(clippy::unwrap_used)]
use super::*;
use crate::{
    control::protocol::Caller, sidebar::layout_tests::fixture_window, state::ConnectionStatus,
};
use std::sync::Arc;

fn call(pane: &str, pinned: bool) -> Call<AgentOp> {
    Call {
        caller: Caller::default(),
        op: AgentOp {
            pane: pane.into(),
            pinned,
        },
    }
}

#[gpui::test]
fn agent_pin_requests_are_idempotent_and_refuse_missing_or_disconnected_agents(
    cx: &mut gpui::TestAppContext,
) {
    let (view, cx) = cx.add_window_view(fixture_window);
    let listing = crate::sidebar::layout_tests::snapshot(2);
    let pane = listing.agents[0].pane_id.clone();
    let second = listing.agents[1].pane_id.clone();
    view.update(cx, |view, _| {
        view.live.snapshot = Some(Arc::new(listing));
        view.live.status = ConnectionStatus::Connected;
    });
    cx.cx.update(|cx| {
        let mut wrong = call(&pane, true);
        wrong.caller.daemon_socket = Some("/tmp/unrelated-herdr-pin-test.sock".into());
        assert!(matches!(
            serve(wrong, cx),
            Response::Error {
                code: ErrorCode::NotFound,
                ..
            }
        ));
    });
    cx.cx.update(|cx| {
        assert!(matches!(
            serve(call("missing", true), cx),
            Response::Error {
                code: ErrorCode::NotFound,
                ..
            }
        ));
        for target in [&pane, &second, &pane] {
            assert_eq!(
                serve(call(target, true), cx),
                Response::Agent {
                    reply: Reply {
                        pane: target.clone(),
                        pinned: true
                    }
                }
            );
        }
    });
    view.update(cx, |view, _| {
        assert_eq!(view.agent_pin_rank(0, &second), Some(0));
        assert_eq!(view.agent_pin_rank(0, &pane), Some(1));
        view.live.status = ConnectionStatus::Detached;
    });
    cx.cx.update(|cx| {
        assert!(matches!(
            serve(call(&pane, false), cx),
            Response::Error {
                code: ErrorCode::NotFound,
                ..
            }
        ));
    });
    view.update(cx, |view, _| view.live.status = ConnectionStatus::Connected);
    cx.cx.update(|cx| {
        assert_eq!(
            serve(call(&pane, false), cx),
            Response::Agent {
                reply: Reply {
                    pane: pane.clone(),
                    pinned: false
                }
            }
        );
        assert!(matches!(
            serve(call(&pane, true), cx),
            Response::Agent { .. }
        ));
    });
    view.update(cx, |view, _| {
        assert_eq!(view.agent_pin_rank(0, &pane), Some(0))
    });
}
