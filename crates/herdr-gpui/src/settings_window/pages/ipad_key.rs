//! This iPad's SSH identity. The first click asks, the second replaces on a
//! worker. Navigating away clears the confirmation but cannot cancel a
//! Keychain operation already in progress.

use super::{SettingsWindow, ui};
use crate::{
    i18n::{Localize, fl},
    settings_window::{
        Section,
        device_key::{self, Line, Operation},
    },
};
use gpui::{prelude::*, *};

#[derive(Default)]
pub(in crate::settings_window) struct IpadKeySettings {
    pub(in crate::settings_window) confirming: bool,
    pub(in crate::settings_window) error: Option<String>,
    pub(in crate::settings_window) line: Line,
    pub(in crate::settings_window) replacing: bool,
    task: Option<Task<()>>,
    #[cfg(test)]
    pub(in crate::settings_window) io: Option<KeyIo>,
}

#[cfg(test)]
type KeyIo = std::sync::Arc<dyn Fn(Operation) -> herdr_client::Result<String> + Send + Sync>;

impl SettingsWindow {
    pub(in crate::settings_window) fn open_ipad_key(&mut self, cx: &mut Context<Self>) {
        if crate::ipad_ui::active() && self.ipad_key.task.is_none() && !self.ipad_key.replacing {
            self.start_ipad_key(Operation::Read, cx);
        }
    }

    pub(in crate::settings_window) fn replace_ipad_key(&mut self, cx: &mut Context<Self>) {
        if !crate::ipad_ui::active()
            || self.section != Section::General
            || self.search_results.is_some()
            || self.ipad_key.task.is_some()
            || self.ipad_key.replacing
            || !matches!(self.ipad_key.line, Line::Ready(_))
        {
            return;
        }
        if !self.ipad_key.confirming {
            self.ipad_key.confirming = true;
            self.ipad_key.error = None;
            cx.notify();
            return;
        }
        self.start_ipad_key(Operation::Replace, cx);
    }

    fn start_ipad_key(&mut self, operation: Operation, cx: &mut Context<Self>) {
        self.ipad_key.confirming = false;
        self.ipad_key.error = None;
        self.ipad_key.replacing = operation == Operation::Replace;
        // A previous public line must not be copied while its replacement runs.
        let previous = std::mem::take(&mut self.ipad_key.line);
        let background = {
            #[cfg(test)]
            if let Some(io) = self.ipad_key.io.clone() {
                cx.background_executor().spawn(async move { io(operation) })
            } else {
                device_key::load(cx.background_executor(), operation)
            }
            #[cfg(not(test))]
            device_key::load(cx.background_executor(), operation)
        };
        // Reopening Settings must reuse this model while promotion is pending,
        // rather than load an old identity into a second model before it commits.
        let retained = (operation == Operation::Replace).then(|| cx.entity());
        let task = cx.spawn(async move |this, cx| {
            let _retained = retained;
            let result = background.await;
            if operation == Operation::Replace && result.is_ok() {
                // A committed key change also invalidates other scenes when
                // the Settings window closed while the worker was running.
                cx.update(device_key::replaced);
            }
            let _ = this.update(cx, |this, cx| {
                this.ipad_key.replacing = false;
                this.ipad_key.task = None;
                match result {
                    Ok(line) => {
                        this.ipad_key.line = Line::Ready(line);
                    }
                    Err(error) => {
                        let text = error.localize();
                        this.ipad_key.error =
                            Some(fl!("settings-ipad-key-failed", error = text.clone()));
                        this.ipad_key.line = if operation == Operation::Replace {
                            // Replacement reports failure only before promotion;
                            // the credentials layer preserves the previous key.
                            previous
                        } else {
                            Line::Failed(text)
                        };
                    }
                }
                cx.notify();
            });
        });
        if operation == Operation::Replace {
            // Keychain promotion cannot be cancelled. Keep its completion
            // alive independently of the window that requested it.
            task.detach();
        } else {
            self.ipad_key.task = Some(task);
        }
        cx.notify();
    }

    pub(in crate::settings_window) fn render_ipad_key(&self, cx: &mut Context<Self>) -> Div {
        let theme = &self.theme;
        let enabled = self.ipad_key.task.is_none()
            && !self.ipad_key.replacing
            && !self.quitting
            && self.closing.is_none();
        let ready = enabled && matches!(self.ipad_key.line, Line::Ready(_));
        let asking = self.ipad_key.confirming;
        let mut key = div().flex().flex_col().gap(px(8.));
        match &self.ipad_key.line {
            Line::Loading => {
                key = key.child(div().text_color(rgb(theme.subtext())).child(
                    if self.ipad_key.replacing {
                        fl!("settings-ipad-key-replacing")
                    } else {
                        fl!("settings-ipad-key-loading")
                    },
                ));
            }
            Line::Failed(_) => {
                key = key.child(
                    ui::button(
                        theme,
                        "ipad-key-retry",
                        fl!("settings-ipad-key-retry"),
                        enabled,
                    )
                    .when(enabled, |button| {
                        button.on_click(cx.listener(|this, _, _, cx| {
                            this.open_ipad_key(cx);
                        }))
                    }),
                );
            }
            Line::Ready(line) => {
                let copied = line.clone();
                key = key
                    .child(
                        div()
                            .debug_selector(|| "general-ipad-key-line".into())
                            .p(px(8.))
                            .rounded(px(crate::config::corners::CONTROL))
                            .bg(rgb(theme.active))
                            .font_family(self.config.terminal.family.clone())
                            .id("ipad-key-line-scroll")
                            .whitespace_nowrap()
                            .overflow_x_scroll()
                            .child(line.clone()),
                    )
                    .child(
                        div().flex().justify_end().child(
                            ui::button(
                                theme,
                                "general-ipad-key-copy",
                                fl!("settings-ipad-key-copy"),
                                ready,
                            )
                            .when(ready, |button| {
                                button.on_click(cx.listener(move |this, _, _, cx| {
                                    if this.ipad_key.replacing
                                        || this.ipad_key.task.is_some()
                                        || !matches!(&this.ipad_key.line, Line::Ready(line) if line == &copied)
                                    {
                                        return;
                                    }
                                    cx.write_to_clipboard(ClipboardItem::new_string(
                                        copied.clone(),
                                    ));
                                    this.status = Some(fl!("settings-ipad-key-copied"));
                                    cx.notify();
                                }))
                            }),
                        ),
                    );
            }
        }
        let error = self.ipad_key.error.clone().or_else(|| {
            if let Line::Failed(error) = &self.ipad_key.line {
                Some(fl!("settings-ipad-key-failed", error = error.clone()))
            } else {
                None
            }
        });
        div()
            .flex()
            .flex_col()
            .child(ui::section(theme, &fl!("settings-ipad-section"), None))
            .child(
                ui::row(theme, "general-ipad-key", fl!("settings-ipad-key"))
                    .description(fl!("settings-ipad-key-description")),
            )
            .child(key)
            .children(error.map(|error| {
                div()
                    .debug_selector(|| "ipad-key-error".into())
                    .text_color(crate::menu::danger(theme))
                    .child(error)
            }))
            .child(
                ui::row(
                    theme,
                    "general-ipad-key-replace",
                    fl!("settings-ipad-key-replace"),
                )
                .description(fl!("settings-ipad-key-replace-description"))
                .when(asking, |row| {
                    row.control(
                        ui::button(
                            theme,
                            "ipad-key-cancel",
                            fl!("settings-ipad-key-cancel"),
                            enabled,
                        )
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.ipad_key.confirming = false;
                            cx.notify();
                        })),
                    )
                })
                .control(
                    ui::button(
                        theme,
                        "ipad-key-replace",
                        if asking {
                            fl!("settings-ipad-key-confirm")
                        } else {
                            fl!("settings-ipad-key-replace-open")
                        },
                        ready,
                    )
                    .text_color(crate::menu::danger(theme))
                    .when(ready, |button| {
                        button.on_click(cx.listener(|this, _, _, cx| {
                            this.replace_ipad_key(cx);
                        }))
                    }),
                ),
            )
            .when(asking, |page| {
                page.child(
                    div()
                        .debug_selector(|| "ipad-key-warning".into())
                        .text_color(crate::menu::danger(theme))
                        .child(fl!("settings-ipad-key-warning")),
                )
            })
    }
}
