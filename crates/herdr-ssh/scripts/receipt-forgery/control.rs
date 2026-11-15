//! Must compile: the same paths `forge.rs` uses resolve from outside the
//! crate, so `forge.rs` fails only because of the private field.

pub fn tunnel_of(receipt: &herdr_ssh::TeardownReceipt) -> herdr_ssh::TunnelId {
    receipt.tunnel()
}
