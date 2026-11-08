//! Configuration warnings prepared for the notice stack.
use super::*;

impl Config {
    /// Warnings naming the keys this build ignored and the configured fonts
    /// that are not installed, one line each, if any. Keys and families come
    /// from the user's file, so each loses the characters untrusted text
    /// does before it joins a message.
    pub(crate) fn diagnostic(&self) -> Option<String> {
        const LISTED: usize = 5;
        const MAX_ITEM_CHARS: usize = 80;
        let list = |items: &[String]| {
            let listed: Vec<_> = items
                .iter()
                .take(LISTED)
                .map(|item| sheprd_client::text::safe_text(item, MAX_ITEM_CHARS))
                .collect();
            (listed.join(", "), items.len().saturating_sub(LISTED))
        };
        let mut lines = Vec::new();
        #[cfg(any(test, target_os = "ios"))]
        ipad_storage::append_migration_warning(self, &mut lines);
        if !self.unknown_keys.is_empty() {
            let (keys, more) = list(&self.unknown_keys);
            lines.push(fl!(
                "config-diagnostic-unknown-keys",
                keys = keys,
                more = more
            ));
        }
        if self.layout.legacy_sidebar_gap.is_some() {
            lines.push(fl!("config-diagnostic-sidebar-gap"));
        }
        if !self.missing_fonts.is_empty() {
            let fonts: Vec<String> = self
                .missing_fonts
                .iter()
                .map(|family| format!("\"{family}\""))
                .collect();
            let (fonts, more) = list(&fonts);
            lines.push(fl!(
                "config-diagnostic-missing-fonts",
                fonts = fonts,
                more = more
            ));
        }
        if let Some(missing) = &self.missing_weight {
            lines.push(fl!(
                "config-diagnostic-missing-weight",
                family = sheprd_client::text::safe_text(&missing.family, MAX_ITEM_CHARS),
                weight = font_weight::describe(missing.weight)
            ));
        }
        (!lines.is_empty()).then(|| lines.join("\n"))
    }
}
