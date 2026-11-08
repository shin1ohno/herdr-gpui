//! A desktop-only window for the public SSH host-key code.

use super::{Code, Error, keys};
use crate::{
    config::{Config, Theme},
    fonts::StyledFont,
    i18n::{Localize, fl},
    remote::qr::{self, Qr},
};
use gpui::{prelude::*, *};
use std::sync::Arc;

pub(super) const QR_SIDE: f32 = 280.;

#[derive(Default)]
struct Handle(Option<WindowHandle<HostKeyWindow>>);
impl Global for Handle {}

pub(crate) fn open(config: Config, theme: Theme, cx: &mut App) {
    cx.defer(move |cx| {
        if let Some(handle) = cx.default_global::<Handle>().0
            && handle
                .update(cx, |view, window, cx| {
                    view.config = config.clone();
                    view.theme = theme.clone();
                    view.load(cx);
                    window.activate_window();
                })
                .is_ok()
        {
            return;
        }
        let bounds = Bounds::centered(None, size(px(680.), px(740.)), cx);
        let opened = cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                window_min_size: Some(size(px(420.), px(440.))),
                titlebar: Some(crate::titlebar::options(&fl!("host-key-code-title"))),
                app_owns_titlebar_drag: cfg!(target_os = "macos"),
                ..Default::default()
            },
            move |window, cx| {
                crate::taskbar::mark(window, cx);
                cx.new(|cx| {
                    let mut view = HostKeyWindow::new(config, theme, window, cx);
                    view.load(cx);
                    view
                })
            },
        );
        match opened {
            Ok(handle) => cx.default_global::<Handle>().0 = Some(handle),
            Err(error) => tracing::error!(%error, "Could not open the host-key code window"),
        }
    });
}

enum State {
    Loading,
    Ready { code: Code, qr: Arc<Qr> },
    Failed(Error),
}

struct HostKeyWindow {
    caption: crate::caption::Caption,
    config: Config,
    focus: FocusHandle,
    state: State,
    task: Option<Task<()>>,
    theme: Theme,
}

impl HostKeyWindow {
    fn new(config: Config, theme: Theme, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let focus = cx.focus_handle();
        window.focus(&focus, cx);
        Self {
            caption: Default::default(),
            config,
            focus,
            state: State::Loading,
            task: None,
            theme,
        }
    }

    fn load(&mut self, cx: &mut Context<Self>) {
        self.state = State::Loading;
        let background = cx.background_executor().spawn(async move {
            let code = keys::read_computer()?;
            let qr = Qr::encode(&code.to_url())?;
            if qr::layout(QR_SIDE, qr.width()).is_none() {
                return Err(Error::TooLarge);
            }
            Ok((code, Arc::new(qr)))
        });
        self.task = Some(cx.spawn(async move |this, cx| {
            let result = background.await;
            let _ = this.update(cx, |this, cx| {
                this.state = match result {
                    Ok((code, qr)) => State::Ready { code, qr },
                    Err(error) => State::Failed(error),
                };
                cx.notify();
            });
        }));
        cx.notify();
    }

    fn body(&self) -> impl IntoElement {
        let mut body = div()
            .id("host-key-code-body")
            .debug_selector(|| "host-key-code-body".into())
            .flex_1()
            .min_h_0()
            .min_w_0()
            .w_full()
            .overflow_y_scroll()
            .flex()
            .flex_col()
            .gap(px(16.))
            .p(px(20.))
            .child(
                div()
                    .flex_none()
                    .text_size(px(self.config.ui.size + 4.))
                    .child(fl!("host-key-code-title")),
            )
            .child(
                div()
                    .flex_none()
                    .whitespace_normal()
                    .child(fl!("host-key-code-intro")),
            );
        match &self.state {
            State::Loading => body.child(
                div()
                    .debug_selector(|| "host-key-code-loading".into())
                    .child(fl!("host-key-code-loading")),
            ),
            State::Failed(error) => body.child(
                div()
                    .debug_selector(|| "host-key-code-error".into())
                    .flex_none()
                    .whitespace_normal()
                    .text_color(crate::menu::danger(&self.theme))
                    .child(error.localize()),
            ),
            State::Ready { code, qr } => {
                body = body.child(
                    div()
                        .debug_selector(|| "host-key-code-qr".into())
                        .flex_none()
                        .flex()
                        .justify_center()
                        .child(crate::remote::qr_square(qr.clone(), QR_SIDE)),
                );
                if let Some(host) = code.host_hint() {
                    body = body.child(
                        div()
                            .flex_none()
                            .whitespace_normal()
                            .child(fl!("host-key-code-computer", host = host)),
                    );
                }
                for key in code.fingerprints() {
                    body = body.child(
                        div()
                            .debug_selector(|| "host-key-code-fingerprint".into())
                            .flex_none()
                            .min_w_0()
                            .w_full()
                            .flex()
                            .flex_col()
                            .gap(px(4.))
                            .font_family(self.config.terminal.family.clone())
                            .child(
                                div()
                                    .text_color(rgb(self.theme.muted))
                                    .child(key.algorithm.clone()),
                            )
                            .child(
                                div()
                                    .min_w_0()
                                    .whitespace_normal()
                                    .child(key.fingerprint.clone()),
                            ),
                    );
                }
                body.child(
                    div()
                        .flex_none()
                        .whitespace_normal()
                        .text_color(rgb(self.theme.muted))
                        .child(fl!("host-key-code-public-only")),
                )
            }
        }
    }
}

impl Render for HostKeyWindow {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.caption.sync(window, &self.theme);
        let header =
            crate::titlebar::header(&self.theme, window, |window, _| window.remove_window());
        let button = |id, text: String| {
            div()
                .id(id)
                .p(px(8.))
                .rounded(px(crate::config::corners::CONTROL))
                .bg(rgb(self.theme.active))
                .cursor_pointer()
                .child(text)
        };
        div()
            .id("host-key-code-window")
            .debug_selector(|| "host-key-code-window".into())
            .role(Role::Dialog)
            .aria_label(fl!("host-key-code-title"))
            .track_focus(&self.focus)
            .on_key_down(cx.listener(|_, event: &KeyDownEvent, window, cx| {
                if event.keystroke.key == "escape" {
                    cx.stop_propagation();
                    window.remove_window();
                }
            }))
            .size_full()
            .flex()
            .flex_col()
            .bg(rgb(self.theme.background))
            .text_color(rgb(self.theme.foreground))
            .text_font(&self.config.ui)
            .text_size(px(self.config.ui.size))
            .line_height(px(self.config.ui.line_height()))
            .children(header)
            .child(self.body())
            .child(
                div()
                    .flex_none()
                    .p(px(16.))
                    .border_t_1()
                    .border_color(rgb(self.theme.active))
                    .flex()
                    .justify_end()
                    .gap(px(8.))
                    .when(matches!(self.state, State::Ready { .. }), |footer| {
                        footer.child(
                            button("host-key-code-copy", fl!("host-key-code-copy"))
                                .debug_selector(|| "host-key-code-copy".into())
                                .on_click(cx.listener(|this, _, _, cx| {
                                    if let State::Ready { code, .. } = &this.state {
                                        cx.write_to_clipboard(ClipboardItem::new_string(
                                            code.to_url(),
                                        ));
                                    }
                                })),
                        )
                    })
                    .child(
                        button("host-key-code-close", fl!("host-key-code-close"))
                            .debug_selector(|| "host-key-code-close".into())
                            .on_click(|_, window, _| window.remove_window()),
                    ),
            )
    }
}

#[cfg(test)]
#[path = "tests/dialog.rs"]
mod tests;
