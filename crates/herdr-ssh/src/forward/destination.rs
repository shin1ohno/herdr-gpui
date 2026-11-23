//! Where on the host a forwarded connection goes: the host's own loopback
//! and a port, and nothing else, so a forward can never reach another
//! machine through the host.

use std::{fmt, num::NonZeroU16, str::FromStr};

use super::Error;

/// A name for the host's loopback interface, sent to the host as the
/// `host to connect` of a `direct-tcpip` channel. OpenSSH matches
/// `permitopen` names literally, so the caller picks the spelling that its
/// `permitopen` allows; `localhost` is what Sheprd's desktop forwards use.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum Loopback {
    // keep-sorted start
    /// `127.0.0.1`.
    Ipv4,
    /// `::1`.
    Ipv6,
    /// `localhost`.
    #[default]
    Localhost,
    // keep-sorted end
}

impl Loopback {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Ipv4 => "127.0.0.1",
            Self::Ipv6 => "::1",
            Self::Localhost => "localhost",
        }
    }
}

impl fmt::Display for Loopback {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl FromStr for Loopback {
    type Err = Error;

    /// Exactly `localhost`, `127.0.0.1`, or `::1` (bracketed or not). Any
    /// other name, address, or spelling is refused rather than resolved, so
    /// the host never decides where a name points.
    fn from_str(host: &str) -> Result<Self, Error> {
        match host {
            "localhost" => Ok(Self::Localhost),
            "127.0.0.1" => Ok(Self::Ipv4),
            "::1" | "[::1]" => Ok(Self::Ipv6),
            _ => Err(Error::NotLoopback),
        }
    }
}

/// A port on the host's loopback.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Destination {
    host: Loopback,
    port: NonZeroU16,
}

impl Destination {
    /// `port` on the host's `localhost`.
    pub fn localhost(port: u16) -> Result<Self, Error> {
        Self::new(Loopback::Localhost, port)
    }

    pub fn new(host: Loopback, port: u16) -> Result<Self, Error> {
        let port = NonZeroU16::new(port).ok_or(Error::InvalidPort)?;
        Ok(Self { host, port })
    }

    /// Parses a host name and a port given separately, refusing any host
    /// that is not a [`Loopback`] name.
    pub fn parse(host: &str, port: u16) -> Result<Self, Error> {
        Self::new(host.parse()?, port)
    }

    pub fn host(self) -> Loopback {
        self.host
    }

    pub fn port(self) -> u16 {
        self.port.get()
    }
}

impl fmt::Display for Destination {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.host {
            Loopback::Ipv6 => write!(formatter, "[::1]:{}", self.port),
            host => write!(formatter, "{host}:{}", self.port),
        }
    }
}
