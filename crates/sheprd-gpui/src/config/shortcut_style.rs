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

impl ShortcutStyle {
    /// The order the Keyboard page presents its choices and columns.
    pub(crate) const ALL: &[Self] = &[Self::Ipad, Self::HerdrPrefix];

    pub(crate) fn name(self) -> &'static str {
        match self {
            Self::Ipad => "ipad",
            Self::HerdrPrefix => "herdr-prefix",
        }
    }

    pub(crate) fn message(self) -> &'static str {
        match self {
            Self::Ipad => "ipad-keyboard-style-ipad",
            Self::HerdrPrefix => "ipad-keyboard-style-herdr-prefix",
        }
    }

    pub(crate) fn label(self) -> String {
        crate::i18n::message(self.message(), &[])
    }
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
