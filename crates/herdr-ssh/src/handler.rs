//! The russh client handler: checks the host key against the pins and
//! refuses everything the bridge never asks for.

use std::sync::{Arc, Mutex, PoisonError};

use russh::{
    ChannelOpenFailure,
    client::{self, DisconnectReason, Msg, Session},
    keys::{PublicKey, PublicKeyOrCertificate},
};

use crate::HostKey;

/// What the host offered during key exchange, compared with the pins.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Offered {
    /// A pinned key.
    Pinned,
    /// A key, and nothing is pinned.
    New(HostKey),
    /// A key that matches no pin, while something is pinned.
    Changed(HostKey),
    /// A host certificate, or a key whose encoding could not be read; never
    /// trusted.
    Refused,
}

/// Shared with the code that started the connection, which reads it after
/// russh has failed or finished the key exchange.
pub(crate) type OfferedSlot = Arc<Mutex<Option<Offered>>>;

pub(crate) fn take(slot: &OfferedSlot) -> Option<Offered> {
    slot.lock().unwrap_or_else(PoisonError::into_inner).take()
}

pub(crate) struct Client {
    pins: Vec<HostKey>,
    /// Whether a pinned key may go on to authentication. A scan refuses
    /// every key after recording it, so it disconnects before the user name
    /// is sent.
    authenticate: bool,
    offered: OfferedSlot,
}

impl Client {
    pub(crate) fn new(pins: Vec<HostKey>, authenticate: bool, offered: OfferedSlot) -> Self {
        Self {
            pins,
            authenticate,
            offered,
        }
    }

    fn compare(&self, key: &PublicKey) -> Offered {
        let Some(key) = key.to_bytes().ok().and_then(HostKey::from_blob) else {
            return Offered::Refused;
        };
        if self.pins.contains(&key) {
            Offered::Pinned
        } else if self.pins.is_empty() {
            Offered::New(key)
        } else {
            Offered::Changed(key)
        }
    }
}

fn refuse(reply: client::ChannelOpenHandle) -> impl Future<Output = ()> + Send {
    reply.reject(ChannelOpenFailure::AdministrativelyProhibited)
}

impl client::Handler for Client {
    type Error = russh::Error;

    /// russh has already verified the exchange signature with this key.
    async fn check_server_key(
        &mut self,
        key: &PublicKeyOrCertificate,
    ) -> Result<bool, Self::Error> {
        let offered = match key {
            PublicKeyOrCertificate::PublicKey { key, .. } => self.compare(key),
            PublicKeyOrCertificate::Certificate(_) => Offered::Refused,
        };
        let accept = self.authenticate && offered == Offered::Pinned;
        *self.offered.lock().unwrap_or_else(PoisonError::into_inner) = Some(offered);
        Ok(accept)
    }

    // Every channel the server opens is refused (`ForwardAgent=no`,
    // `ForwardX11=no`, `ClearAllForwardings=yes`): the bridge requests no
    // forwarding, so none is expected, and accepting would let the host
    // open channels without bound.

    async fn server_channel_open_forwarded_tcpip(
        &mut self,
        _channel: russh::Channel<Msg>,
        _connected_address: &str,
        _connected_port: u32,
        _originator_address: &str,
        _originator_port: u32,
        reply: client::ChannelOpenHandle,
        _session: &mut Session,
    ) -> Result<(), Self::Error> {
        refuse(reply).await;
        Ok(())
    }

    async fn server_channel_open_forwarded_streamlocal(
        &mut self,
        _channel: russh::Channel<Msg>,
        _socket_path: &str,
        reply: client::ChannelOpenHandle,
        _session: &mut Session,
    ) -> Result<(), Self::Error> {
        refuse(reply).await;
        Ok(())
    }

    async fn server_channel_open_agent_forward(
        &mut self,
        _channel: russh::Channel<Msg>,
        reply: client::ChannelOpenHandle,
        _session: &mut Session,
    ) -> Result<(), Self::Error> {
        refuse(reply).await;
        Ok(())
    }

    async fn should_accept_unknown_server_channel(
        &mut self,
        _id: russh::ChannelId,
        _channel_type: &str,
    ) -> bool {
        false
    }

    async fn server_channel_open_session(
        &mut self,
        _channel: russh::Channel<Msg>,
        reply: client::ChannelOpenHandle,
        _session: &mut Session,
    ) -> Result<(), Self::Error> {
        refuse(reply).await;
        Ok(())
    }

    async fn server_channel_open_direct_tcpip(
        &mut self,
        _channel: russh::Channel<Msg>,
        _host_to_connect: &str,
        _port_to_connect: u32,
        _originator_address: &str,
        _originator_port: u32,
        reply: client::ChannelOpenHandle,
        _session: &mut Session,
    ) -> Result<(), Self::Error> {
        refuse(reply).await;
        Ok(())
    }

    async fn server_channel_open_direct_streamlocal(
        &mut self,
        _channel: russh::Channel<Msg>,
        _socket_path: &str,
        reply: client::ChannelOpenHandle,
        _session: &mut Session,
    ) -> Result<(), Self::Error> {
        refuse(reply).await;
        Ok(())
    }

    async fn server_channel_open_x11(
        &mut self,
        _channel: russh::Channel<Msg>,
        _originator_address: &str,
        _originator_port: u32,
        reply: client::ChannelOpenHandle,
        _session: &mut Session,
    ) -> Result<(), Self::Error> {
        refuse(reply).await;
        Ok(())
    }

    /// `UpdateHostKeys=no`: announced keys are neither proved nor pinned.
    async fn openssh_ext_host_keys_announced(
        &mut self,
        _keys: Vec<PublicKey>,
        _session: &mut Session,
    ) -> Result<(), Self::Error> {
        Ok(())
    }

    /// As russh's default, without its debug log of the server's
    /// disconnect message.
    async fn disconnected(
        &mut self,
        reason: DisconnectReason<Self::Error>,
    ) -> Result<(), Self::Error> {
        match reason {
            DisconnectReason::ReceivedDisconnect(_) => Ok(()),
            DisconnectReason::Error(error) => Err(error),
        }
    }
}

#[cfg(test)]
mod tests;
