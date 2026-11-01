//! Dispatch the saved host menu's actions; terminal creation is explicit.
use super::{Action, Page, SearchInput};
#[cfg(feature = "cloud")]
use super::{HostMenu, Saved};
use crate::HerdrWindow;
use gpui::{prelude::*, *};

impl HerdrWindow {
    pub(super) fn activate_host_menu(
        &mut self,
        action: Action,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        tracing::info!(
            category = "device_remove",
            ?action,
            id = self.menu.host.as_ref().map(|host| host.id.as_str()),
            "Host menu action"
        );
        match action {
            #[cfg(target_os = "ios")]
            Action::NewCoreTerminal => {
                let Some(id) = self.menu.host.as_ref().map(|host| host.id.clone()) else {
                    return;
                };
                self.dismiss_menu(window, cx);
                self.new_core_terminal_for_host(&id, window, cx);
                return;
            }
            Action::Rename => {
                let Some(host) = &self.menu.host else {
                    return;
                };
                let (label, target) = (host.label.clone(), host.target.clone());
                let input = cx.new(SearchInput::new);
                input.update(cx, |input, cx| {
                    input.set_appearance(self.config.ui.clone(), self.theme.clone(), cx);
                    // An empty name falls back to the target, as when adding.
                    input.set_placeholder(&target, cx);
                    input.set_text_selected(&label, cx);
                    window.focus(&input.focus, cx);
                });
                if let Some(host) = &mut self.menu.host {
                    host.input = Some(input);
                    host.error = None;
                }
                self.menu.page = Some(Page::RenameDevice);
            }
            Action::ServerKeybindings => self.toggle_server_keybindings(cx),
            Action::Theme => self.open_device_theme(window, cx),
            #[cfg(feature = "cloud")]
            Action::ProviderSettings => {
                if let Some(HostMenu {
                    saved: Saved::Cloud { provider, .. },
                    ..
                }) = self.menu.host
                {
                    self.open_provider_settings(provider, window, cx);
                }
                return;
            }
            #[cfg(feature = "cloud")]
            Action::RunSetup => {
                let Some(id) = self.menu.host.as_ref().map(|host| host.id.clone()) else {
                    return;
                };
                self.dismiss_menu(window, cx);
                if let Err(error) = self.rerun_cloud_setup(&id, cx) {
                    self.show_flash(crate::window::Flash::warning(error.to_string()), cx);
                }
                return;
            }
            #[cfg(all(unix, not(target_os = "ios")))]
            Action::ForwardPort => {
                let input = cx.new(SearchInput::new);
                input.update(cx, |input, cx| {
                    input.set_appearance(self.config.ui.clone(), self.theme.clone(), cx);
                    input.set_placeholder("3000", cx);
                    window.focus(&input.focus, cx);
                });
                if let Some(host) = &mut self.menu.host {
                    host.input = Some(input);
                    host.error = None;
                }
                self.menu.page = Some(Page::ForwardPort);
            }
            Action::Remove => {
                self.menu.page = Some(Page::RemoveDevice);
                #[cfg(feature = "cloud")]
                self.check_unsynced(cx);
            }
        }
        cx.notify();
    }
}
