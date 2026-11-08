//! Newest-first pins belong to a host and daemon boot, never to a row title.
use sheprd_client::protocol::ClientShellSnapshot;

const MAX_PINS: usize = 1024;
const MAX_ID: usize = 4096;

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub(crate) struct AgentPin {
    endpoint: String,
    boot: String,
    pane: String,
}

impl AgentPin {
    pub(crate) fn new(endpoint: &str, boot: &str, pane: &str) -> Self {
        Self {
            endpoint: endpoint.into(),
            boot: boot.into(),
            pane: pane.into(),
        }
    }

    fn valid(&self) -> bool {
        [&self.endpoint, &self.boot, &self.pane]
            .into_iter()
            .all(|id| !id.is_empty() && id.len() <= MAX_ID)
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize)]
pub(crate) struct AgentPins(Vec<AgentPin>);

impl AgentPins {
    pub(crate) fn read(value: Option<&serde_json::Value>) -> Self {
        let mut pins = Self::default();
        for value in value
            .and_then(serde_json::Value::as_array)
            .into_iter()
            .flatten()
            .take(MAX_PINS)
        {
            if let Ok(pin) = serde_json::from_value::<AgentPin>(value.clone())
                && pin.valid()
                && !pins.0.contains(&pin)
            {
                pins.0.push(pin);
            }
        }
        pins
    }

    pub(crate) fn rank(&self, endpoint: &str, boot: &str, pane: &str) -> Option<usize> {
        self.0
            .iter()
            .position(|pin| pin.endpoint == endpoint && pin.boot == boot && pin.pane == pane)
    }

    pub(crate) fn toggle(&mut self, pin: AgentPin) {
        if let Some(index) = self.0.iter().position(|stored| *stored == pin) {
            self.0.remove(index);
        } else if pin.valid() {
            self.0.insert(0, pin);
            self.0.truncate(MAX_PINS);
        }
    }

    /// A disconnected snapshot is not evidence of closure. A connected
    /// snapshot can retire closed panes of its own boot; other sessions stay.
    pub(crate) fn reconcile(&mut self, endpoint: &str, snapshot: &ClientShellSnapshot) -> bool {
        let before = self.0.len();
        self.0.retain(|pin| {
            pin.endpoint != endpoint
                || pin.boot != snapshot.boot_id
                || snapshot.panes.iter().any(|pane| pane.pane_id == pin.pane)
        });
        self.0.len() != before
    }
}

#[cfg(test)]
mod tests;
