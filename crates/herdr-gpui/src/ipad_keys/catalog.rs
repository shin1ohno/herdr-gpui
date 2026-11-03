use crate::i18n::fl;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Binding {
    // keep-sorted start
    Alt,
    Control,
    Hide,
    Keystroke(&'static str),
    More,
    Paste,
    Text(&'static str),
    // keep-sorted end
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Side {
    // keep-sorted start
    Compact,
    Left,
    Right,
    // keep-sorted end
}

macro_rules! keys {
    ($($key:ident => ($label:expr, $binding:expr, $width:expr, $side:ident, $compact:expr)),+ $(,)?) => {
        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        pub(super) enum Key { $($key),+ }
        impl Key {
            pub(super) const ALL: &[Self] = &[$(Self::$key),+];
            pub(super) fn label(self) -> String {
                match self { $(Self::$key => $label),+ }
            }
            pub(super) fn binding(self) -> Binding {
                match self { $(Self::$key => $binding),+ }
            }
            pub(super) fn width(self) -> f32 {
                match self { $(Self::$key => $width),+ }
            }
            pub(super) fn side(self) -> Side {
                match self { $(Self::$key => Side::$side),+ }
            }
            pub(super) fn compact(self) -> Option<u8> {
                match self { $(Self::$key => $compact),+ }
            }
        }
    };
}

// Order is the full accessory band's reading order, with More in compact.
keys! {
    Escape => (fl!("ipad-keys-key-esc"), Binding::Keystroke("escape"), 52., Left, Some(0)),
    Tab => (fl!("ipad-keys-key-tab"), Binding::Keystroke("tab"), 52., Left, Some(2)),
    Control => (fl!("ipad-keys-key-ctrl"), Binding::Control, 52., Left, Some(1)),
    Alt => (fl!("ipad-keys-key-alt"), Binding::Alt, 52., Left, None),
    Left => (fl!("ipad-keys-key-left"), Binding::Keystroke("left"), 48., Left, None),
    Up => (fl!("ipad-keys-key-up"), Binding::Keystroke("up"), 48., Left, Some(3)),
    Down => (fl!("ipad-keys-key-down"), Binding::Keystroke("down"), 48., Left, Some(4)),
    Right => (fl!("ipad-keys-key-right"), Binding::Keystroke("right"), 48., Left, None),
    Pipe => (fl!("ipad-keys-key-pipe"), Binding::Text("|"), 44., Right, None),
    Tilde => (fl!("ipad-keys-key-tilde"), Binding::Text("~"), 44., Right, None),
    Slash => (fl!("ipad-keys-key-slash"), Binding::Text("/"), 44., Right, None),
    Dash => (fl!("ipad-keys-key-dash"), Binding::Text("-"), 44., Right, None),
    Paste => (fl!("ipad-keys-key-paste"), Binding::Paste, 52., Right, None),
    Hide => (fl!("ipad-keys-key-hide"), Binding::Hide, 52., Right, None),
    More => (fl!("ipad-keys-key-more"), Binding::More, 44., Compact, Some(5)),
}
