#[derive(Debug, thiserror::Error)]
pub enum Error {
    // keep-sorted start
    #[error(transparent)]
    Client(#[from] sheprd_client::Error),
    #[error(transparent)]
    Core(#[from] sheprd_core_client::Error),
    #[cfg(target_os = "ios")]
    #[error("Could not configure the SSH core stream")]
    Io(#[from] std::io::Error),
    #[error("This terminal belongs to a different core host or session.")]
    Namespace,
    #[error("This build has no herdr-core beside it; install herdr-core from releases.herdr.dev")]
    #[cfg(not(target_os = "ios"))]
    NotBundled,
    #[cfg(target_os = "ios")]
    #[error("The remote hc connect command ended ({exit_status:?})")]
    RemoteCommand { exit_status: Option<u32> },
    #[cfg(target_os = "ios")]
    #[error(transparent)]
    Ssh(#[from] herdr_ssh::Failure),
    #[error("Herdr 2 terminals are unavailable on iPad until a remote core is connected.")]
    Unavailable,
    #[error("Herdr 2 terminals are unavailable for the selected endpoint on this platform.")]
    UnsupportedTarget,
    #[error("Could not start a worker for the terminal: {0}")]
    Worker(#[source] std::io::Error),
    // keep-sorted end
}
