use super::*;
use crate::keymap::Reach;

// The full reference remains available; the transient guide must stay cheap
// even when a host advertises a long command list.
const MAX_GUIDE_COMMANDS: usize = 32;

impl SheprdWindow {
    pub(super) fn custom_guide_section(
        &self,
        style: ShortcutStyle,
        armed: bool,
        cx: &mut Context<Self>,
    ) -> Option<Div> {
        let snapshot = self
            .live
            .snapshot
            .as_ref()
            .filter(|_| self.live.status.is_connected())?;
        let commands = &snapshot.commands[..snapshot.commands.len().min(MAX_GUIDE_COMMANDS)];
        let bindings = self.keymap().custom_bindings(commands);
        let rows: Vec<_> = commands
            .iter()
            .zip(bindings)
            .filter_map(|(command, bindings)| {
                let bound = bindings.into_iter().find(|binding| {
                    binding.reach == Reach::Runs
                        && binding.label.contains(' ') == (style == ShortcutStyle::HerdrPrefix)
                })?;
                let label_keys = glyphs(&bound.label);
                let key = if armed {
                    label_keys.split_once(' ')?.1
                } else {
                    &label_keys
                };
                let id = command.command_id.clone();
                let action = command.action;
                let label: String = command
                    .description
                    .as_deref()
                    .unwrap_or(&id)
                    .chars()
                    .take(160)
                    .collect();
                Some(
                    div()
                        .id(SharedString::from(format!("shortcut-guide-custom-{id}")))
                        .role(Role::Button)
                        .aria_label(label.clone())
                        .w_full()
                        .h(px(44.))
                        .flex_none()
                        .flex()
                        .items_center()
                        .gap(px(12.))
                        .px(px(12.))
                        .rounded(px(8.))
                        .cursor_pointer()
                        .hover(|row| row.bg(rgb(self.theme.active)))
                        .child(kbd(key.to_owned(), &self.theme, &self.config.ui))
                        .child(div().min_w_0().truncate().child(label))
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.close_shortcut_guide(cx);
                            this.invoke_custom_command(&id, action, cx);
                        })),
                )
            })
            .collect();
        if rows.is_empty() {
            return None;
        }
        Some(
            div()
                .flex_1()
                .min_w(px(220.))
                .flex()
                .flex_col()
                .gap(px(4.))
                .child(
                    div()
                        .px(px(12.))
                        .py(px(8.))
                        .text_color(rgb(self.theme.subtext()))
                        .child(fl!("shortcut-guide-custom")),
                )
                .children(rows),
        )
    }
}
