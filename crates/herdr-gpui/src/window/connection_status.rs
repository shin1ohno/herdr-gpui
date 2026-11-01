//! Connection presentation follows the backend being shown. Core state never
//! becomes a Herdr 1 snapshot or grants access to its command queue.

use crate::{HerdrWindow, endpoint::Endpoint, i18n::fl, state::ConnectionStatus};

impl HerdrWindow {
    pub(crate) fn uses_core_terminal_backend(&self) -> bool {
        self.shows_core_connection(&self.endpoints[self.selected_endpoint])
    }

    fn shows_core_connection(&self, endpoint: &Endpoint) -> bool {
        cfg!(target_os = "ios")
            && (endpoint.backend_generation() == herdr_backend::Generation::Herdr2
                || (endpoint.id == self.endpoints[self.selected_endpoint].id
                    && self.core_workspace_key().is_some()))
    }

    pub(crate) fn displayed_connection_status(&self, endpoint: &Endpoint) -> ConnectionStatus {
        if self.shows_core_connection(endpoint) {
            self.core_connection_status(&endpoint.id)
        } else if endpoint.id == self.endpoints[self.selected_endpoint].id {
            self.live.status
        } else {
            endpoint.live.status
        }
    }

    pub(super) fn selected_connection_text(&self) -> String {
        let endpoint = &self.endpoints[self.selected_endpoint];
        if !self.shows_core_connection(endpoint) {
            if cfg!(target_os = "ios") && endpoint.id == crate::endpoint::LOCAL {
                return fl!("status-bar-ipad-select-device");
            }
            return self.live.status_text(self.local_error.as_deref());
        }
        let status = self.core_connection_status(&endpoint.id);
        let error = if status.is_connected() {
            self.local_error.clone()
        } else {
            self.core_connection_error(&endpoint.id)
                .map(ToString::to_string)
        };
        match error {
            Some(error) => fl!(
                "status-bar-status-error",
                status = status.label(),
                error = error
            ),
            None => status.label(),
        }
    }
}

#[cfg(test)]
mod tests;
