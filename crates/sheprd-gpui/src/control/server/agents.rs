//! Pinning changes only the client-local Agents panel, through its usual owner.
use crate::control::protocol::{
    Call, ErrorCode, Response,
    agents::{AgentOp, Reply},
};
use gpui::App;

pub(super) fn serve(call: Call<AgentOp>, cx: &mut App) -> Response {
    super::in_free_window(&call.caller, cx, |view, _, cx| {
        if call
            .caller
            .daemon_socket
            .as_deref()
            .is_some_and(|socket| !view.shows_daemon(std::path::Path::new(socket)))
        {
            return super::not_found("The selected host is not the caller's daemon");
        }
        if !view.live.status.is_connected() {
            return Response::error(ErrorCode::NotFound, "The agent host is disconnected");
        }
        let Some(snapshot) = view.live.snapshot.as_ref() else {
            return super::not_found("No agent snapshot");
        };
        if !snapshot
            .agents
            .iter()
            .any(|agent| agent.pane_id == call.op.pane)
        {
            return super::not_found("No such agent in this window's selected host");
        }
        if let Err(error) = view.set_agent_pin(&call.op.pane, call.op.pinned) {
            return Response::error(ErrorCode::NotFound, error.to_string());
        }
        cx.notify();
        Response::Agent {
            reply: Reply {
                pane: call.op.pane,
                pinned: call.op.pinned,
            },
        }
    })
}

#[cfg(test)]
mod tests;
