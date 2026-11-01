//! The system Bonjour daemon owns multicast on iOS. Every operation here
//! runs on the discovery worker; callbacks only enqueue bounded results.
//! References share one socket, polled at most 100 ms before cancellation
//! is checked. No blocking system hostname resolver is used.

mod error;
mod ffi;
#[cfg(test)]
mod tests;

pub use error::Error;

use super::{Candidate, candidate};
use ffi::{Browser, Event, Service};
use std::{
    collections::{HashMap, HashSet},
    net::IpAddr,
    time::{Duration, Instant},
};

const POLL: Duration = Duration::from_millis(100);
const RETRY: Duration = Duration::from_secs(1);
const MAX_SERVICES: usize = super::super::MAX_SUGGESTIONS;
const MAX_ADDRESSES: usize = super::super::MAX_KEYS;

#[derive(Default)]
struct Peer {
    addresses: Vec<IpAddr>,
    host: Option<String>,
    local: bool,
    name: String,
    port: u16,
}

impl Peer {
    fn address(&mut self, address: IpAddr, add: bool, local: &HashSet<IpAddr>) {
        if add {
            self.local |= local.contains(&address);
            if self.addresses.len() < MAX_ADDRESSES && !self.addresses.contains(&address) {
                self.addresses.push(address);
            }
        } else {
            self.addresses.retain(|stored| *stored != address);
        }
    }

    fn candidate(&self, local: &HashSet<IpAddr>) -> Option<Candidate> {
        if self.local || self.addresses.is_empty() {
            return None;
        }
        candidate(
            &self.name,
            self.host.as_deref()?,
            self.port,
            &self.addresses,
            local,
        )
    }
}

pub(super) fn browse(
    duration: Duration,
    permission_wait: Duration,
    mut waiting: impl FnMut(bool) -> bool,
    mut found: impl FnMut(Candidate) -> bool,
    local: &HashSet<IpAddr>,
) -> crate::Result<()> {
    let permission_deadline = Instant::now() + permission_wait;
    let mut denied = false;
    loop {
        if !waiting(denied) {
            return Ok(());
        }
        let result = run(duration, &mut waiting, &mut found, local);
        match result {
            Ok(()) => return Ok(()),
            Err(error) if error.denied() => {
                if Instant::now() >= permission_deadline {
                    return Err(crate::Error::LocalNetworkDenied);
                }
                denied = true;
                // PolicyDenied also occurs while the permission alert is
                // awaiting an answer. Recreate the browse after a bounded
                // pause; every slice checks whether the form still exists.
                let retry = (Instant::now() + RETRY).min(permission_deadline);
                while Instant::now() < retry {
                    if !waiting(true) {
                        return Ok(());
                    }
                    std::thread::sleep(POLL.min(retry.saturating_duration_since(Instant::now())));
                }
            }
            Err(error) => return Err(crate::menu::devices::Error::from(error).into()),
        }
    }
}

fn run(
    duration: Duration,
    waiting: &mut impl FnMut(bool) -> bool,
    found: &mut impl FnMut(Candidate) -> bool,
    local: &HashSet<IpAddr>,
) -> Result<(), Error> {
    let mut browser = Browser::new()?;
    let deadline = Instant::now() + duration;
    let mut seen = HashSet::<Service>::new();
    let mut peers = HashMap::<usize, Peer>::new();
    let mut ids = HashMap::<Service, usize>::new();
    while Instant::now() < deadline && waiting(false) {
        browser.process(POLL.min(deadline.saturating_duration_since(Instant::now())))?;
        let mut changed = HashSet::new();
        while let Some(event) = browser.next() {
            match event {
                Event::Service { service, add } => {
                    if !add {
                        if let Some(id) = ids.remove(&service) {
                            peers.remove(&id);
                            browser.cancel(id);
                        }
                    } else if !ids.contains_key(&service)
                        && (seen.contains(&service) || seen.len() < MAX_SERVICES)
                    {
                        seen.insert(service.clone());
                        let id = browser.resolve(&service)?;
                        peers.insert(
                            id,
                            Peer {
                                name: service.name(),
                                ..Peer::default()
                            },
                        );
                        ids.insert(service, id);
                    }
                }
                Event::Resolved {
                    id,
                    host,
                    port,
                    interface,
                } => {
                    if let Some(peer) = peers.get_mut(&id) {
                        peer.host = Some(host.to_string_lossy().into_owned());
                        peer.port = port;
                        browser.addresses(id, interface, &host)?;
                    }
                }
                Event::Address { id, address, add } => {
                    if let Some(peer) = peers.get_mut(&id) {
                        peer.address(address, add, local);
                        changed.insert(id);
                    }
                }
                Event::Failed { id, error } => {
                    if id == 0 || error.denied() {
                        return Err(error);
                    }
                    // One vanished service, or an absent address family,
                    // does not invalidate other announced machines. Keep
                    // the bounded query alive: an IPv4 failure must not
                    // cancel IPv6 results (or vice versa).
                }
            }
            if !waiting(false) {
                return Ok(());
            }
        }
        // MoreComing is shared by every child of a shared connection, not
        // one hostname. Flush every changed peer after processing the batch.
        for id in changed {
            if let Some(candidate) = peers.get(&id).and_then(|peer| peer.candidate(local))
                && !found(candidate)
            {
                return Ok(());
            }
        }
    }
    Ok(())
}
