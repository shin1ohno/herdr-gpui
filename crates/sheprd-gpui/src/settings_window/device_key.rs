//! The public SSH line shared by Add Device and Settings. Keychain work
//! always runs on the background executor, including key replacement.

use crate::i18n::Localize;
use gpui::{App, BackgroundExecutor, Global, Task};

/// Public-key views must refresh after Settings changes the shared identity.
#[derive(Default)]
pub(crate) struct Revision;
impl Global for Revision {}

pub(super) fn replaced(cx: &mut App) {
    cx.set_global(Revision);
}

#[derive(Default)]
pub(crate) enum Line {
    #[default]
    Loading,
    Ready(String),
    /// Localized display text; the worker keeps the typed error until here.
    Failed(String),
}

impl From<sheprd_client::Result<String>> for Line {
    fn from(result: sheprd_client::Result<String>) -> Self {
        match result {
            Ok(line) => Self::Ready(line),
            Err(error) => Self::Failed(error.localize()),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Operation {
    Read,
    Replace,
}

pub(crate) fn load(
    executor: &BackgroundExecutor,
    operation: Operation,
) -> Task<sheprd_client::Result<String>> {
    executor.spawn(async move {
        #[cfg(target_os = "ios")]
        match operation {
            Operation::Read => sheprd_client::authorized_key(),
            Operation::Replace => sheprd_client::replace_authorized_key(),
        }
        #[cfg(not(target_os = "ios"))]
        {
            // Host tests inject their own worker. Never access a host Keychain.
            let _ = operation;
            Err(sheprd_client::ssh::Error::Unsupported.into())
        }
    })
}
