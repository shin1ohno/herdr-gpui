use super::SheprdWindow;
use crate::{controls::Command, keymap::glyphs};

impl SheprdWindow {
    pub(crate) fn shortcut_hint(&self, command: Command) -> Option<String> {
        self.keymap()
            .styled(command, self.config.shortcut_style)
            .map(glyphs)
    }
}
