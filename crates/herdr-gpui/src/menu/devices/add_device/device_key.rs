//! On iPadOS, this device's own SSH key in the Add Device form: the line a
//! computer needs in `~/.ssh/authorized_keys` before it lets the iPad in, with
//! a Copy button (docs/ios.md, "Client key"). The key lives in the Keychain
//! and is created on first use, so it is read off the UI thread.

use super::Setup;
use crate::HerdrWindow;
use crate::i18n::fl;
use crate::window::Flash;
use gpui::{prelude::*, *};

/// This device's `authorized_keys` line, while the form is open.
#[derive(Default)]
pub(in crate::menu) struct DeviceKey {
    line: Line,
    /// Held so the read is not cancelled; dropping the form drops it.
    _task: Option<Task<()>>,
}

#[derive(Default)]
enum Line {
    #[default]
    Loading,
    Ready(String),
    /// Why the key could not be read, such as a locked device.
    Failed(String),
}

impl HerdrWindow {
    /// Reads the line for the open form, creating the key on first use.
    pub(super) fn load_device_key(&mut self, cx: &mut Context<Self>) {
        let Some(setup) = &mut self.menu.device_setup else {
            return;
        };
        let background = cx
            .background_executor()
            .spawn(async { herdr_client::authorized_key() });
        setup.device_key._task = Some(cx.spawn(async move |this, cx| {
            let result = background.await;
            let _ = this.update(cx, |this, cx| {
                let Some(setup) = &mut this.menu.device_setup else {
                    return;
                };
                setup.device_key.line = match result {
                    Ok(line) => Line::Ready(line),
                    Err(error) => {
                        tracing::warn!(category = "device_setup", "Device key unavailable");
                        Line::Failed(error.to_string())
                    }
                };
                cx.notify();
            });
        }));
    }

    pub(super) fn render_device_key(
        &self,
        setup: &Setup,
        cx: &mut Context<Self>,
    ) -> impl IntoElement + use<> {
        let theme = &self.theme;
        let mut section = div()
            .debug_selector(|| "device-setup-device-key".into())
            .flex_none()
            .flex()
            .flex_col()
            .gap(px(6.))
            .child(fl!("add-device-ipad-key"));
        match &setup.device_key.line {
            Line::Loading => {
                section = section.child(
                    div()
                        .text_color(rgb(theme.muted))
                        .child(fl!("add-device-ipad-key-loading")),
                );
            }
            Line::Failed(error) => {
                section = section.child(
                    div()
                        .text_color(crate::menu::danger(theme))
                        .child(fl!("add-device-ipad-key-failed", error = error.clone())),
                );
            }
            Line::Ready(line) => {
                let copied = line.clone();
                section = section
                    .child(
                        div()
                            .debug_selector(|| "device-setup-device-key-line".into())
                            .p(px(8.))
                            .rounded(px(crate::config::corners::CONTROL))
                            .bg(rgb(theme.active))
                            .font_family(self.config.terminal.family.clone())
                            .child(line.clone()),
                    )
                    .child(
                        div().flex().justify_end().child(
                            div()
                                .id("device-setup-device-key-copy")
                                .debug_selector(|| "device-setup-device-key-copy".into())
                                .p(px(6.))
                                .rounded(px(crate::config::corners::CONTROL))
                                .bg(rgb(theme.active))
                                .cursor_pointer()
                                .child(fl!("add-device-ipad-key-copy"))
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    cx.write_to_clipboard(ClipboardItem::new_string(
                                        copied.clone(),
                                    ));
                                    this.show_flash(
                                        Flash::success(fl!("add-device-ipad-key-copied")),
                                        cx,
                                    );
                                })),
                        ),
                    );
            }
        }
        section
    }
}
