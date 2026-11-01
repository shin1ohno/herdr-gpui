use serde::{Deserialize, Serialize};

/// Which resolved bindings the iPad's shortcut hints show.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum ShortcutStyle {
    // keep-sorted start
    HerdrPrefix,
    Ipad,
    // keep-sorted end
}

impl Default for ShortcutStyle {
    fn default() -> Self {
        if cfg!(target_os = "ios") {
            Self::Ipad
        } else {
            Self::HerdrPrefix
        }
    }
}
