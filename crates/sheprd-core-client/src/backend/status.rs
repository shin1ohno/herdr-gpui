//! A terminal's status as herdr-core reported it, before `TerminalInfo`
//! folds it into the few states every backend shares.

use super::{CoreBackend, CoreChanges};
use crate::{
    Result,
    wire::{Status, Terminal},
};

/// One terminal's reported status, unfolded: `TerminalInfo::agent` maps
/// error to blocked and every unknown state to unknown, so a consumer that
/// tells them apart reads this instead. `status` is `None` when the
/// terminal's program has reported nothing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoreStatus {
    /// The terminal's address, the same string as its `TerminalId`.
    pub address: String,
    pub status: Option<Status>,
}

impl From<&Terminal> for CoreStatus {
    fn from(terminal: &Terminal) -> Self {
        Self {
            address: terminal.address.clone(),
            status: terminal.status.clone(),
        }
    }
}

impl CoreBackend {
    /// Every listed terminal's reported status, from the same `core.list`
    /// as `terminals()`.
    pub fn statuses(&self) -> Result<Vec<CoreStatus>> {
        Ok(self
            .core
            .list(false)?
            .terminals
            .iter()
            .map(CoreStatus::from)
            .collect())
    }
}

impl CoreChanges {
    /// Every terminal's reported status with the events read so far applied,
    /// in address order.
    pub fn statuses(&self) -> Vec<CoreStatus> {
        self.records.values().map(CoreStatus::from).collect()
    }

    /// One terminal's reported status after the events read so far, or
    /// `None` when the terminal is unknown or was removed.
    pub fn status(&self, address: &str) -> Option<CoreStatus> {
        self.records.get(address).map(CoreStatus::from)
    }
}
