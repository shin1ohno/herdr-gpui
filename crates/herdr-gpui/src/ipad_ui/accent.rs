use crate::config::Theme;
use std::sync::LazyLock;

// Theme carries colors, not a name. Compare the built-in palette rather than
// display labels; contrast and system-accent changes leave this identity intact.
static NORD: LazyLock<Option<Theme>> = LazyLock::new(|| Theme::builtin("Nord"));

pub(crate) fn accent(theme: &Theme) -> u32 {
    if super::layout()
        && NORD
            .as_ref()
            .is_some_and(|nord| theme.palette[..16] == nord.palette[..16])
    {
        theme.palette[6]
    } else {
        theme.primary()
    }
}
