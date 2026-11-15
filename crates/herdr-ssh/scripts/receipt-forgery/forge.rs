//! Must not compile: outside herdr-ssh a `TeardownReceipt` cannot be built
//! (rule R10). Every field is listed, so adding a field makes rustc reject
//! the literal for another reason (without E0451), and
//! `check-receipt-forgery.sh` then fails until this file lists it too. Only
//! the struct literal is checked here: a public constructor, `Default`, or
//! `Clone` added to `TeardownReceipt` is for review to catch.

pub fn forge(tunnel: herdr_ssh::TunnelId) -> herdr_ssh::TeardownReceipt {
    herdr_ssh::TeardownReceipt { tunnel }
}
