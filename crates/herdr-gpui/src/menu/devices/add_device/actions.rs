//! The SSH form's step actions, shared by its desktop footer and iPad header.

use super::{Offer, Setup, Step};
use crate::{HerdrWindow, i18n::fl};
use gpui::{prelude::*, *};

pub(super) struct StepActions {
    pub(super) cancel: bool,
    pub(super) primary: Option<Primary>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Primary {
    // keep-sorted start
    Checking,
    #[cfg(target_os = "ios")]
    Core,
    Setup(Offer),
    Submit,
    Trust,
    // keep-sorted end
}

pub(super) fn step_actions(step: &Step) -> StepActions {
    let (cancel, primary) = match step {
        #[cfg(target_os = "ios")]
        Step::Confirm(_, Offer::CoreAvailable | Offer::OutdatedCoreAvailable) => {
            (true, Some(Primary::Core))
        }
        Step::Confirm(..) if cfg!(any(windows, target_os = "ios")) => (true, None),
        Step::Confirm(_, offer) => (true, Some(Primary::Setup(*offer))),
        Step::AwaitingLocalNetwork(_) | Step::KeyCodeRefused => (true, None),
        Step::TrustKey(..) => (true, Some(Primary::Trust)),
        Step::Form => (false, Some(Primary::Submit)),
        Step::SavedWarning(_) => (false, None),
        _ => (false, Some(Primary::Checking)),
    };
    StepActions { cancel, primary }
}

impl Primary {
    pub(super) fn label(self) -> String {
        match self {
            Self::Submit => fl!("add-device-submit"),
            Self::Checking => fl!("add-device-submit-checking"),
            Self::Setup(offer) => offer.action(),
            Self::Trust => fl!("add-device-trust-key"),
            #[cfg(target_os = "ios")]
            Self::Core => fl!("add-device-core-add"),
        }
    }

    pub(super) fn id(self) -> &'static str {
        match self {
            Self::Trust => "device-setup-trust",
            #[cfg(target_os = "ios")]
            Self::Core => "device-setup-add-core",
            _ => "device-setup-submit",
        }
    }

    fn enabled(self, setup: &Setup) -> bool {
        match self {
            Self::Checking => false,
            Self::Submit => !setup.host_key_code.needs_confirmation(),
            _ => true,
        }
    }
}

impl HerdrWindow {
    pub(super) fn device_action_button(
        &self,
        id: &'static str,
        label: impl Into<SharedString>,
        enabled: bool,
        primary: bool,
    ) -> Stateful<Div> {
        let theme = &self.theme;
        let accent = crate::ipad_ui::accent(theme);
        div()
            .id(id)
            .debug_selector(move || id.into())
            .p(px(8.))
            .rounded(px(crate::config::corners::CONTROL))
            .bg(rgb(theme.active))
            .when(enabled, |button| {
                button.cursor_pointer().hover(|s| {
                    if crate::ipad_ui::layout() && primary {
                        s.bg(rgb(accent))
                    } else {
                        s.bg(rgb(theme.active).blend(rgba((theme.foreground << 8) | 0x20)))
                    }
                })
            })
            .when(!enabled, |button| button.text_color(rgb(theme.muted)))
            .when(crate::ipad_ui::layout(), |button| {
                button
                    .min_h(px(crate::ipad_ui::metrics::TOUCH))
                    .min_w(px(crate::ipad_ui::metrics::TOUCH))
                    .flex()
                    .items_center()
                    .justify_center()
                    .when(primary && enabled, |button| {
                        button
                            .bg(rgb(accent))
                            .text_color(rgb(theme.text_on(accent)))
                    })
                    .when(!enabled, |button| button.text_color(rgb(theme.subtext())))
            })
            .child(label.into())
    }

    pub(super) fn render_device_primary(
        &self,
        action: Primary,
        id: &'static str,
        setup: &Setup,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        self.device_action_button(id, action.label(), action.enabled(setup), true)
            .on_click(cx.listener(move |this, _, _, cx| match action {
                Primary::Submit | Primary::Checking => this.submit_device_setup(cx),
                Primary::Setup(_) => this.open_setup_space(cx),
                Primary::Trust => this.trust_host_key(cx),
                #[cfg(target_os = "ios")]
                Primary::Core => this.confirm_core_device(cx),
            }))
    }
}
