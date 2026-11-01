//! Keeping the machine awake: `error-caffeine.ftl`.

use super::Localize;
use crate::{caffeine::Error, i18n::fl};

impl Localize for Error {
    fn localize(&self) -> String {
        match self {
            // keep-sorted start
            // The system's own words.
            #[cfg(not(target_os = "ios"))]
            Self::Hold(error) => fl!("error-caffeine-hold", error = error.to_string()),
            #[cfg(target_os = "ios")]
            Self::MainThread => fl!("error-caffeine-main-thread"),
            Self::NoWorkingAgents => fl!("error-caffeine-no-working-agents"),
            #[cfg(not(target_os = "ios"))]
            Self::Thread(_) => fl!("error-caffeine-thread"),
            #[cfg(target_os = "ios")]
            Self::UnsupportedScope => fl!("error-caffeine-unsupported-scope"),
            #[cfg(not(target_os = "ios"))]
            Self::WorkerStopped => fl!("error-caffeine-worker-stopped"),
            // keep-sorted end
        }
    }
}
