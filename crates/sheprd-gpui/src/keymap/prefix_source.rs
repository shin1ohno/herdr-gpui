//! Provenance travels with the prefix parsed from the daemon's configuration.
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum LocalPrefixSource {
    Default(Option<PathBuf>),
    Configured(Option<PathBuf>),
}

impl LocalPrefixSource {
    pub(super) fn parsed(configured: bool) -> Self {
        if configured {
            Self::Configured(None)
        } else {
            Self::Default(None)
        }
    }

    pub(super) fn set_path(&mut self, path: &Path) {
        match self {
            Self::Default(location) | Self::Configured(location) => {
                *location = Some(path.to_owned())
            }
        }
    }

    pub(crate) fn path(&self) -> Option<&Path> {
        match self {
            Self::Default(path) | Self::Configured(path) => path.as_deref(),
        }
    }
}

/// The same answer used for input dispatch and source presentation. A server
/// profile is already normalized; it cannot prove which remote values were
/// explicitly written, so its source remains the host's published profile.
pub(crate) enum KeymapSource<'a> {
    Local(&'a LocalPrefixSource),
    Server(&'a str),
}

pub(crate) struct ResolvedKeymap<'a> {
    pub(crate) keymap: &'a super::Keymap,
    pub(crate) source: KeymapSource<'a>,
}
