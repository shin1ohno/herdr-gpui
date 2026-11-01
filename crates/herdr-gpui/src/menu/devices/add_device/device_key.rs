//! On iPadOS, this device's own SSH key in the Add Device form: the line a
//! computer needs in `~/.ssh/authorized_keys` before it lets the iPad in, with
//! a Copy button (docs/ios.md, "Client key"). The key lives in the Keychain
//! and is created on first use, so it is read off the UI thread.

use super::Setup;
use crate::HerdrWindow;
use crate::i18n::fl;
use crate::settings_window::device_key::{self, Line, Operation};
use crate::window::Flash;
use gpui::{prelude::*, *};

/// This device's `authorized_keys` line, while the form is open.
#[derive(Default)]
pub(in crate::menu) struct DeviceKey {
    line: Line,
    /// An open form must stop offering a key Settings has just retired.
    _changed: Option<Subscription>,
    /// Held so the read is not cancelled; dropping the form drops it.
    _task: Option<Task<()>>,
}

impl HerdrWindow {
    /// Reads the line for the open form, creating the key on first use.
    pub(super) fn load_device_key(&mut self, cx: &mut Context<Self>) {
        let Some(setup) = &mut self.menu.device_setup else {
            return;
        };
        if setup.device_key._changed.is_none() {
            setup.device_key._changed = Some(
                cx.observe_global::<device_key::Revision>(|this, cx| this.load_device_key(cx)),
            );
        }
        setup.device_key.line = Line::Loading;
        let background = device_key::load(cx.background_executor(), Operation::Read);
        setup.device_key._task = Some(cx.spawn(async move |this, cx| {
            let result = background.await;
            let _ = this.update(cx, |this, cx| {
                let Some(setup) = &mut this.menu.device_setup else {
                    return;
                };
                setup.device_key.line = result.into();
                cx.notify();
            });
        }));
        cx.notify();
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
                                    // A replacement can invalidate this painted
                                    // button before the next frame redraws it.
                                    if !this.menu.device_setup.as_ref().is_some_and(|setup| {
                                        matches!(&setup.device_key.line, Line::Ready(line) if line == &copied)
                                    }) {
                                        return;
                                    }
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
