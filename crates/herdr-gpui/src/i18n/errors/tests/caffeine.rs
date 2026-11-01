use super::*;
use crate::caffeine::Error;

#[cfg(not(target_os = "ios"))]
#[test]
fn every_caffeine_variant_has_a_message() {
    assert_localized(&[
        Error::Hold(keepawake::Error::Builder(
            keepawake::BuilderError::ValidationError("no assertion".into()),
        )),
        Error::NoWorkingAgents,
        Error::Thread(std::io::Error::other("no threads")),
        Error::WorkerStopped,
    ]);
}

#[cfg(target_os = "ios")]
#[test]
fn every_ipad_caffeine_variant_has_a_message() {
    assert_localized(&[
        Error::MainThread,
        Error::NoWorkingAgents,
        Error::UnsupportedScope,
    ]);
}
