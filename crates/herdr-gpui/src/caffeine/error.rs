//! Why the machine could not be kept awake.

#[derive(Debug, thiserror::Error)]
pub enum Error {
    // keep-sorted start
    #[cfg(not(target_os = "ios"))]
    #[error("Could not keep the machine awake: {0}")]
    Hold(#[source] keepawake::Error),
    #[cfg(target_os = "ios")]
    #[error("The display idle timer must be changed on the main thread")]
    MainThread,
    #[error("No agent is working, so there is nothing to stay awake for")]
    NoWorkingAgents,
    #[cfg(not(target_os = "ios"))]
    #[error("Could not start the keep-awake thread")]
    Thread(#[source] std::io::Error),
    #[cfg(target_os = "ios")]
    #[error(
        "Keeping only the system awake is unavailable on iPad; choose display keep-awake in Settings"
    )]
    UnsupportedScope,
    #[cfg(not(target_os = "ios"))]
    #[error("The keep-awake thread stopped; the display may sleep")]
    WorkerStopped,
    // keep-sorted end
}
