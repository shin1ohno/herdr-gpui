//! `user@host[:port]`, parsed once at the boundary and then shared by scan,
//! trust, connect, and the device list (docs/ios.md, "Targets").
//!
//! The grammar is deliberately narrow, because the host part becomes the
//! name host keys are pinned under: two spellings of one host must give one
//! name, or a pinned host would look unknown and ask the user again.
//!
//! - The user is required (the iPad's own user name is never sent), at most
//!   [`USER_LIMIT`] bytes of visible ASCII without `@` or `:`, and does not
//!   start with `-`.
//! - A host name is ASCII letters, digits, `-`, and `.` in labels of 1-63
//!   characters that neither start nor end with `-`, at most 253 characters;
//!   it is lower-cased and one trailing `.` is removed. Non-ASCII names must
//!   be written in punycode (`xn--…`).
//! - An IPv6 address is written only in brackets, `[addr]` or `[addr]:port`,
//!   without a zone id, and is stored in its canonical (RFC 5952) form.
//! - The port is decimal, 1-65535, and defaults to 22.
//! - `ssh://` URLs, whitespace, and commas are refused.

use std::net::Ipv6Addr;

use crate::{Error, Result};

/// The longest user name accepted, in bytes.
const USER_LIMIT: usize = 256;
/// The longest host name, as DNS allows it without the trailing dot.
const HOST_LIMIT: usize = 253;
const LABEL_LIMIT: usize = 63;
const DEFAULT_PORT: u16 = 22;

/// A parsed SSH target. Host names are ASCII lower-case without a trailing
/// dot; IPv6 addresses come only as `[addr]:port`, without zone ids.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Target {
    pub user: String,
    pub host: String,
    pub port: u16,
}

impl Target {
    /// Parses and normalizes `user@host`, `user@host:port`, or
    /// `user@[v6addr]:port`. The user is required.
    pub fn parse(text: &str) -> Result<Self> {
        if text.starts_with("ssh://") {
            return Err(invalid("write the target as user@host or user@host:port"));
        }
        if text.chars().any(|c| c.is_whitespace() || c.is_control()) {
            return Err(invalid(
                "the target contains whitespace or control characters",
            ));
        }
        if text.contains(',') {
            return Err(invalid("the target contains a comma"));
        }
        // A user name may not contain `@`, so the first one separates it.
        let Some((user, rest)) = text.split_once('@') else {
            return Err(invalid("the target needs a user, as in user@host"));
        };
        validate_user(user)?;
        let (host, port) = split_host_port(rest)?;
        Ok(Self {
            user: user.to_owned(),
            host,
            port,
        })
    }

    /// The name host keys are pinned under: `host` on port 22, otherwise
    /// `[host]:port`, as OpenSSH writes it.
    pub fn known_hosts_name(&self) -> String {
        if self.port == DEFAULT_PORT {
            self.host.clone()
        } else {
            format!("[{}]:{}", self.host, self.port)
        }
    }
}

fn invalid(reason: &'static str) -> Error {
    Error::InvalidTarget { reason }
}

fn validate_user(user: &str) -> Result<()> {
    if user.is_empty() {
        return Err(invalid("the target needs a user, as in user@host"));
    }
    if user.len() > USER_LIMIT {
        return Err(invalid("the user name is too long"));
    }
    if user.starts_with('-') {
        return Err(invalid("the user name starts with '-'"));
    }
    if !user.bytes().all(|b| b.is_ascii_graphic() && b != b':') {
        return Err(invalid(
            "the user name may contain only visible ASCII characters other than ':'",
        ));
    }
    Ok(())
}

/// Splits `host`, `host:port`, `[v6]`, or `[v6]:port` and normalizes the host.
fn split_host_port(rest: &str) -> Result<(String, u16)> {
    if let Some(bracketed) = rest.strip_prefix('[') {
        let Some((address, after)) = bracketed.split_once(']') else {
            return Err(invalid("an IPv6 address is missing its closing ']'"));
        };
        let port = match after {
            "" => DEFAULT_PORT,
            _ => match after.strip_prefix(':') {
                Some(port) => parse_port(port)?,
                None => return Err(invalid("only ':port' may follow an IPv6 address")),
            },
        };
        return Ok((normalize_ipv6(address)?, port));
    }
    if rest.contains(['[', ']']) {
        return Err(invalid("brackets may only enclose an IPv6 address"));
    }
    let (host, port) = match rest.split_once(':') {
        None => (rest, DEFAULT_PORT),
        Some((_, after)) if after.contains(':') => {
            return Err(invalid("write an IPv6 address in brackets, as in [::1]:22"));
        }
        Some((host, port)) => (host, parse_port(port)?),
    };
    Ok((normalize_host_name(host)?, port))
}

fn normalize_ipv6(address: &str) -> Result<String> {
    if address.contains('%') {
        return Err(invalid("IPv6 zone ids are not supported"));
    }
    address
        .parse::<Ipv6Addr>()
        .map(|address| address.to_string())
        .map_err(|_| invalid("the brackets do not contain an IPv6 address"))
}

fn normalize_host_name(host: &str) -> Result<String> {
    if !host.is_ascii() {
        return Err(invalid("write a non-ASCII host name in punycode (xn--)"));
    }
    let host = host.strip_suffix('.').unwrap_or(host);
    if host.is_empty() {
        return Err(invalid("the target has no host"));
    }
    if host.len() > HOST_LIMIT {
        return Err(invalid("the host name is too long"));
    }
    for label in host.split('.') {
        if label.is_empty() {
            return Err(invalid("the host name has an empty label"));
        }
        if label.len() > LABEL_LIMIT {
            return Err(invalid("a host name label is longer than 63 characters"));
        }
        if label.starts_with('-') || label.ends_with('-') {
            return Err(invalid("a host name label starts or ends with '-'"));
        }
        if !label
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-')
        {
            return Err(invalid(
                "a host name may contain only letters, digits, '-', and '.'",
            ));
        }
    }
    Ok(host.to_ascii_lowercase())
}

fn parse_port(port: &str) -> Result<u16> {
    if port.is_empty() || !port.bytes().all(|b| b.is_ascii_digit()) {
        return Err(invalid("the port must be a number from 1 to 65535"));
    }
    match port.parse::<u16>() {
        Ok(0) | Err(_) => Err(invalid("the port must be a number from 1 to 65535")),
        Ok(port) => Ok(port),
    }
}

#[cfg(test)]
mod tests;
