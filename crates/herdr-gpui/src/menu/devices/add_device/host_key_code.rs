//! A host-key code is a transient assertion for one Add Device target. URL
//! delivery is unauthenticated, so it remains pending until explicitly
//! confirmed; typing a code is itself an explicit action.

use super::{Setup, Step, setup};
#[cfg(any(target_os = "ios", test))]
use crate::menu::Page;
use crate::{
    HerdrWindow,
    host_key_code::Code,
    i18n::{Localize, fl},
    search_input::SearchInput,
};
use gpui::{prelude::*, *};
use herdr_client::HostKeyFingerprint;

#[derive(Default)]
pub(in crate::menu::devices) struct Attempt {
    input: Option<Entity<SearchInput>>,
    expected: Option<(String, Code)>,
    pending: Option<Code>,
    matched: bool,
}

impl Attempt {
    pub(super) fn needs_confirmation(&self) -> bool {
        self.pending.is_some()
    }

    pub(super) fn is_composing(&self, cx: &App) -> bool {
        self.input
            .as_ref()
            .is_some_and(|input| input.read(cx).is_composing())
    }

    pub(super) fn target_changed(&mut self, target: &str, cx: &mut App) -> bool {
        if self
            .expected
            .as_ref()
            .is_some_and(|(bound, _)| bound != target)
        {
            self.clear(cx);
            return true;
        }
        false
    }

    fn clear(&mut self, cx: &mut App) {
        self.expected = None;
        self.pending = None;
        self.matched = false;
        if let Some(input) = &self.input {
            input.update(cx, |input, cx| input.clear(cx));
        }
    }

    fn expected(&self, target: &str) -> Option<&Code> {
        self.expected
            .as_ref()
            .filter(|(bound, _)| bound == target)
            .map(|(_, code)| code)
    }
}

impl Setup {
    pub(in crate::menu::devices) fn clear_host_key_code(&mut self, cx: &mut App) {
        self.host_key_code.clear(cx);
    }
}

impl HerdrWindow {
    pub(super) fn init_device_code_field(&mut self, cx: &mut Context<Self>) {
        if !crate::ipad_ui::active() {
            return;
        }
        let input = cx.new(SearchInput::new);
        input.update(cx, |input, cx| {
            input.set_appearance(self.config.ui.clone(), self.theme.clone(), cx);
            input.set_placeholder_with(|| fl!("host-key-code-paste-placeholder"), cx);
        });
        if let Some(form) = &mut self.menu.device_setup {
            form.host_key_code.input = Some(input);
        }
    }

    /// Called only by the iPad URL handler. Never commits an expected set.
    #[cfg(any(target_os = "ios", test))]
    pub(crate) fn receive_host_key_code_url(
        &mut self,
        text: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !crate::ipad_ui::active() {
            return;
        }
        if self.menu.page != Some(Page::AddDevice) {
            if self.menu.page.is_some() || !self.open_menu(window, cx) {
                return;
            }
            self.open_add_device(window, cx);
        }
        let Some(form) = &mut self.menu.device_setup else {
            return;
        };
        if !matches!(form.step, Step::Form)
            || form.host_key_code.expected.is_some()
            || form.host_key_code.pending.is_some()
        {
            return;
        }
        form.clear_host_key_code(cx);
        match Code::parse(text) {
            Ok(code) => {
                form.host_key_code.pending = Some(code);
                form.error = None;
            }
            Err(error) => form.error = Some(error.localize()),
        }
        cx.notify();
    }

    pub(in crate::menu::devices) fn has_device_host_key_code(&self, target: &str) -> bool {
        self.menu
            .device_setup
            .as_ref()
            .is_some_and(|form| form.host_key_code.expected(target).is_some())
    }

    pub(in crate::menu::devices) fn device_key_code_matches(
        &self,
        target: &str,
        offered: &[HostKeyFingerprint],
    ) -> Option<bool> {
        self.menu
            .device_setup
            .as_ref()?
            .host_key_code
            .expected(target)
            .map(|code| code.matches(offered))
    }

    pub(in crate::menu::devices) fn device_key_code_mismatch(
        &self,
        target: &str,
        offered: &[HostKeyFingerprint],
    ) -> String {
        let expected = self
            .menu
            .device_setup
            .as_ref()
            .and_then(|form| form.host_key_code.expected(target))
            .map(code_fingerprints)
            .unwrap_or_default();
        let offered = offered
            .iter()
            .map(|key| format!("{} {} {}", key.kind, key.bits, key.fingerprint))
            .collect::<Vec<_>>()
            .join("\n");
        fl!(
            "host-key-code-mismatch",
            target = target,
            expected = expected,
            offered = offered
        )
    }

    pub(in crate::menu::devices) fn mark_device_code_matched(&mut self) {
        if let Some(form) = &mut self.menu.device_setup {
            form.host_key_code.matched = true;
        }
    }

    /// Input-change subscriptions are delivered after their entity update.
    /// An arriving worker result must fence that interval as well.
    pub(in crate::menu::devices) fn withdraw_stale_device_code(
        &mut self,
        cx: &mut Context<Self>,
    ) -> bool {
        let Some(form) = &mut self.menu.device_setup else {
            return false;
        };
        let target = form.fields[0].read(cx).text().trim().to_owned();
        if !form.host_key_code.target_changed(&target, cx) {
            return false;
        }
        form.step = Step::Form;
        form.task = None;
        form.claim = None;
        form.error = None;
        cx.notify();
        true
    }

    fn accept_device_host_key_code(&mut self, pasted: bool, cx: &mut Context<Self>) {
        let Some(form) = &mut self.menu.device_setup else {
            return;
        };
        if !matches!(form.step, Step::Form) {
            return;
        }
        let target = form.fields[0].read(cx).text().trim().to_owned();
        if target.is_empty() {
            form.error = Some(fl!("host-key-code-enter-target"));
            cx.notify();
            return;
        }
        let code = if pasted {
            let Some(input) = &form.host_key_code.input else {
                return;
            };
            match Code::parse(input.read(cx).text()) {
                Ok(code) => code,
                Err(error) => {
                    form.error = Some(error.localize());
                    cx.notify();
                    return;
                }
            }
        } else {
            let Some(code) = form.host_key_code.pending.take() else {
                return;
            };
            code
        };
        form.clear_host_key_code(cx);
        form.host_key_code.expected = Some((target, code));
        form.error = None;
        cx.notify();
    }

    fn accept_device_code_and_focus(
        &mut self,
        pasted: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.accept_device_host_key_code(pasted, cx);
        if let Some(form) = &self.menu.device_setup
            && form.host_key_code.expected.is_some()
        {
            window.focus(&form.fields[0].read(cx).focus.clone(), cx);
        }
    }

    fn dismiss_device_host_key_code(&mut self, cx: &mut Context<Self>) {
        if let Some(form) = &mut self.menu.device_setup {
            if !matches!(form.step, Step::Form) {
                return;
            }
            form.clear_host_key_code(cx);
            form.error = None;
        }
        cx.notify();
    }

    pub(super) fn apply_focused_host_key_code(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let focused = self.menu.device_setup.as_ref().is_some_and(|form| {
            matches!(form.step, Step::Form)
                && form.host_key_code.expected.is_none()
                && !form.host_key_code.needs_confirmation()
                && form
                    .host_key_code
                    .input
                    .as_ref()
                    .is_some_and(|input| input.read(cx).focus.is_focused(window))
        });
        if focused {
            self.accept_device_code_and_focus(true, window, cx);
        }
        focused
    }

    pub(super) fn focus_host_key_code_field(
        &self,
        backwards: bool,
        window: &mut Window,
        cx: &mut App,
    ) -> bool {
        let Some(form) = &self.menu.device_setup else {
            return false;
        };
        if !matches!(form.step, Step::Form)
            || form.host_key_code.expected.is_some()
            || form.host_key_code.pending.is_some()
        {
            return false;
        }
        let Some(input) = &form.host_key_code.input else {
            return false;
        };
        let from = if backwards { 0 } else { 2 };
        if form.fields[from].read(cx).focus.is_focused(window) {
            window.focus(&input.read(cx).focus.clone(), cx);
        } else if input.read(cx).focus.is_focused(window) {
            window.focus(
                &form.fields[if backwards { 2 } else { 0 }]
                    .read(cx)
                    .focus
                    .clone(),
                cx,
            );
        } else {
            return false;
        }
        true
    }

    pub(super) fn render_device_host_key_code(&self, form: &Setup, cx: &mut Context<Self>) -> Div {
        let state = &form.host_key_code;
        let mut section = div()
            .debug_selector(|| "host-key-code-device".into())
            .flex_none()
            .min_w_0()
            .w_full()
            .flex()
            .flex_col()
            .gap(px(8.))
            .whitespace_normal();
        let button = |id: &'static str, label: String| {
            div()
                .id(id)
                .debug_selector(move || id.into())
                .p(px(8.))
                .rounded(px(crate::config::corners::CONTROL))
                .bg(rgb(self.theme.active))
                .cursor_pointer()
                .child(label)
        };
        if let Some(code) = &state.pending {
            section = section
                .child(
                    div()
                        .debug_selector(|| "host-key-code-confirm-warning".into())
                        .child(fl!("host-key-code-confirm-warning")),
                )
                .child(code_summary(code));
            if matches!(form.step, Step::Form) {
                section = section.child(
                    div()
                        .flex()
                        .flex_wrap()
                        .gap(px(8.))
                        .child(
                            button("host-key-code-accept", fl!("host-key-code-accept")).on_click(
                                cx.listener(|this, _, window, cx| {
                                    this.accept_device_code_and_focus(false, window, cx)
                                }),
                            ),
                        )
                        .child(
                            button("host-key-code-dismiss", fl!("host-key-code-dismiss")).on_click(
                                cx.listener(|this, _, _, cx| this.dismiss_device_host_key_code(cx)),
                            ),
                        ),
                );
            }
        } else if let Some((_, code)) = &state.expected {
            section = section
                .child(fl!("host-key-code-ready"))
                .child(code_summary(code));
            if state.matched {
                section = section.child(
                    div()
                        .debug_selector(|| "host-key-code-matched".into())
                        .child(fl!("host-key-code-matched")),
                );
            }
            if matches!(form.step, Step::Form) {
                section = section.child(
                    button("host-key-code-clear", fl!("host-key-code-clear")).on_click(
                        cx.listener(|this, _, _, cx| this.dismiss_device_host_key_code(cx)),
                    ),
                );
            }
        } else if matches!(form.step, Step::Form) {
            section = section
                .child(fl!("host-key-code-paste-label"))
                .children(state.input.clone())
                .child(
                    button("host-key-code-use-pasted", fl!("host-key-code-use-pasted")).on_click(
                        cx.listener(|this, _, window, cx| {
                            this.accept_device_code_and_focus(true, window, cx)
                        }),
                    ),
                );
        }
        section
    }

    /// A supplied code is checked before authentication, including for a
    /// host that already has a pin. The latter can never enter the save path.
    pub(super) fn start_device_code_scan(
        &mut self,
        request: setup::Request,
        cx: &mut Context<Self>,
    ) {
        let Some(form) = &mut self.menu.device_setup else {
            return;
        };
        form.step = Step::FetchingKey(request.clone());
        form.error = None;
        let background = cx.background_executor().spawn(async move {
            let claim = setup::claim(&request)?;
            Ok::<_, crate::Error>((claim, herdr_client::scan_host_key(request.target())))
        });
        form.task = Some(cx.spawn(async move |this, cx| {
            let result = background.await;
            let _ = this.update(cx, |this, cx| {
                let Some(form) = &mut this.menu.device_setup else {
                    return;
                };
                if !matches!(form.step, Step::FetchingKey(_)) {
                    return;
                }
                match result {
                    Ok((claim, scan)) => {
                        form.claim = Some(claim);
                        this.host_key_fetched(scan, cx);
                    }
                    Err(error) => {
                        form.step = Step::Form;
                        form.error = Some(error.localize());
                        cx.notify();
                    }
                }
            });
        }));
        cx.notify();
    }
}

fn code_summary(code: &Code) -> Div {
    let mut summary = div()
        .flex_none()
        .min_w_0()
        .w_full()
        .flex()
        .flex_col()
        .gap(px(4.))
        .whitespace_normal();
    if let Some(hint) = code.host_hint() {
        summary = summary.child(fl!("host-key-code-host-hint", host = hint));
    }
    for fingerprint in code.fingerprints() {
        summary = summary.child(format!(
            "{} {}",
            fingerprint.algorithm(),
            fingerprint.fingerprint()
        ));
    }
    summary
}

fn code_fingerprints(code: &Code) -> String {
    code.fingerprints()
        .iter()
        .map(|key| format!("{} {}", key.algorithm(), key.fingerprint()))
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests;
