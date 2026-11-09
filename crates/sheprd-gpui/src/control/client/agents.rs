use super::{Context, Output, unexpected};
use crate::control::protocol::{Request, Response, agents::AgentOp};

pub(super) fn agent(pane: String, pinned: bool, context: &Context) -> crate::Result<Output> {
    match context.call(Request::Agent, AgentOp { pane, pinned })? {
        Response::Agent { reply } => Output::data(
            format!(
                "{}: {}",
                reply.pane,
                if reply.pinned { "pinned" } else { "unpinned" }
            ),
            &reply,
        ),
        response => Err(unexpected(response)),
    }
}
