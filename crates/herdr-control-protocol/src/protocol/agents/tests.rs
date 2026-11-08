#![allow(clippy::unwrap_used)]
use super::*;
use crate::protocol::{Call, Caller, Request, Response};

#[test]
fn agent_pin_request_and_response_roundtrip_with_window_targeting() {
    let request = Request::Agent(Call {
        caller: Caller {
            window: Some(7),
            ..Default::default()
        },
        op: AgentOp {
            pane: "p0".into(),
            pinned: true,
        },
    });
    assert_eq!(
        serde_json::from_slice::<Request>(&serde_json::to_vec(&request).unwrap()).unwrap(),
        request
    );
    let response = Response::Agent {
        reply: Reply {
            pane: "p0".into(),
            pinned: true,
        },
    };
    assert_eq!(
        serde_json::from_slice::<Response>(&serde_json::to_vec(&response).unwrap()).unwrap(),
        response
    );
    assert!(
        serde_json::from_str::<AgentOp>(r#"{"pane":"p0","pinned":true,"host":"wrong"}"#).is_err()
    );
}
