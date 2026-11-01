#![allow(clippy::unwrap_used)]

use super::*;

fn parsed(text: &str) -> (String, String, u16) {
    let target = Target::parse(text).unwrap();
    (target.user, target.host, target.port)
}

fn refused(text: &str) -> &'static str {
    match Target::parse(text) {
        Err(Error::InvalidTarget { reason }) => reason,
        other => panic!("{text:?} was not refused: {other:?}"),
    }
}

fn name(text: &str) -> String {
    Target::parse(text).unwrap().known_hosts_name()
}

#[test]
fn host_name_with_default_port() {
    assert_eq!(
        parsed("alice@mac.local"),
        ("alice".into(), "mac.local".into(), 22)
    );
}

#[test]
fn host_name_with_port() {
    assert_eq!(
        parsed("alice@mac.local:2222"),
        ("alice".into(), "mac.local".into(), 2222)
    );
}

#[test]
fn host_names_are_lower_cased() {
    assert_eq!(parsed("Alice@Mac.LOCAL").1, "mac.local");
    // The user is a remote account name, kept as written.
    assert_eq!(parsed("Alice@Mac.LOCAL").0, "Alice");
}

#[test]
fn one_trailing_dot_is_removed() {
    assert_eq!(parsed("a@mac.local.").1, "mac.local");
    assert_eq!(
        parsed("a@mac.local.:2222"),
        ("a".into(), "mac.local".into(), 2222)
    );
    assert_eq!(refused("a@mac.local.."), "the host name has an empty label");
    assert_eq!(refused("a@."), "the target has no host");
}

#[test]
fn spellings_of_one_host_share_a_known_hosts_name() {
    assert_eq!(name("a@Mac.Local."), name("b@mac.local:22"));
    assert_eq!(name("a@MAC.local:022"), "mac.local");
}

#[test]
fn known_hosts_name_brackets_other_ports() {
    assert_eq!(name("a@mac.local"), "mac.local");
    assert_eq!(name("a@mac.local:2222"), "[mac.local]:2222");
    assert_eq!(name("a@10.0.0.2:22"), "10.0.0.2");
    assert_eq!(name("a@[::1]"), "::1");
    assert_eq!(name("a@[::1]:22"), "::1");
    assert_eq!(name("a@[::1]:2200"), "[::1]:2200");
}

#[test]
fn ipv6_in_brackets_is_canonical() {
    assert_eq!(parsed("a@[::1]:2222"), ("a".into(), "::1".into(), 2222));
    assert_eq!(parsed("a@[::1]"), ("a".into(), "::1".into(), 22));
    assert_eq!(parsed("a@[FE80:0:0:0:0:0:0:0001]:22").1, "fe80::1");
    assert_eq!(parsed("a@[2001:DB8::0:1]").1, "2001:db8::1");
    assert_eq!(parsed("a@[::ffff:192.0.2.1]").1, "::ffff:192.0.2.1");
}

#[test]
fn ipv6_needs_brackets() {
    assert_eq!(
        refused("a@::1"),
        "write an IPv6 address in brackets, as in [::1]:22"
    );
    assert_eq!(
        refused("a@fe80::1:22"),
        "write an IPv6 address in brackets, as in [::1]:22"
    );
}

#[test]
fn ipv6_bracket_errors() {
    assert_eq!(
        refused("a@[::1"),
        "an IPv6 address is missing its closing ']'"
    );
    assert_eq!(
        refused("a@[::1]22"),
        "only ':port' may follow an IPv6 address"
    );
    assert_eq!(
        refused("a@[::1]:"),
        "the port must be a number from 1 to 65535"
    );
    assert_eq!(
        refused("a@[mac.local]"),
        "the brackets do not contain an IPv6 address"
    );
    assert_eq!(
        refused("a@[192.0.2.1]"),
        "the brackets do not contain an IPv6 address"
    );
    assert_eq!(
        refused("a@[]"),
        "the brackets do not contain an IPv6 address"
    );
    assert_eq!(
        refused("a@mac]"),
        "brackets may only enclose an IPv6 address"
    );
    assert_eq!(
        refused("a@ma[c"),
        "brackets may only enclose an IPv6 address"
    );
    assert_eq!(
        refused("a@[::1]]:22"),
        "only ':port' may follow an IPv6 address"
    );
}

#[test]
fn ipv6_zone_ids_are_refused() {
    assert_eq!(
        refused("a@[fe80::1%en0]:22"),
        "IPv6 zone ids are not supported"
    );
    assert_eq!(
        refused("a@[fe80::1%25en0]"),
        "IPv6 zone ids are not supported"
    );
}

#[test]
fn user_is_required() {
    assert_eq!(
        refused("mac.local"),
        "the target needs a user, as in user@host"
    );
    assert_eq!(
        refused("@mac.local"),
        "the target needs a user, as in user@host"
    );
}

#[test]
fn user_characters() {
    assert_eq!(parsed("first.last_1-x@mac").0, "first.last_1-x");
    assert_eq!(
        refused("-oProxyCommand@mac"),
        "the user name starts with '-'"
    );
    assert_eq!(
        refused("user:pass@mac"),
        "the user name may contain only visible ASCII characters other than ':'"
    );
    assert_eq!(
        refused("ユーザー@mac"),
        "the user name may contain only visible ASCII characters other than ':'"
    );
    assert_eq!(
        refused(&format!("{}@mac", "u".repeat(257))),
        "the user name is too long"
    );
    assert_eq!(parsed(&format!("{}@mac", "u".repeat(256))).0.len(), 256);
}

#[test]
fn second_at_sign_is_part_of_the_host_and_refused() {
    assert_eq!(
        refused("a@b@mac"),
        "a host name may contain only letters, digits, '-', and '.'"
    );
}

#[test]
fn host_is_required() {
    assert_eq!(refused("a@"), "the target has no host");
    assert_eq!(refused("a@:22"), "the target has no host");
}

#[test]
fn non_ascii_hosts_need_punycode() {
    assert_eq!(
        refused("a@例え.jp"),
        "write a non-ASCII host name in punycode (xn--)"
    );
    assert_eq!(
        refused("a@mäc.local"),
        "write a non-ASCII host name in punycode (xn--)"
    );
    assert_eq!(parsed("a@xn--r8jz45g.jp").1, "xn--r8jz45g.jp");
}

#[test]
fn host_name_characters_and_labels() {
    let charset = "a host name may contain only letters, digits, '-', and '.'";
    assert_eq!(refused("a@mac_book.local"), charset);
    assert_eq!(refused("a@mac*.local"), charset);
    assert_eq!(refused("a@mac/x"), charset);
    assert_eq!(
        refused("a@-mac.local"),
        "a host name label starts or ends with '-'"
    );
    assert_eq!(
        refused("a@mac-.local"),
        "a host name label starts or ends with '-'"
    );
    assert_eq!(refused("a@mac..local"), "the host name has an empty label");
    assert_eq!(refused("a@.mac"), "the host name has an empty label");
    let long_label = "x".repeat(64);
    assert_eq!(
        refused(&format!("a@{long_label}.local")),
        "a host name label is longer than 63 characters"
    );
    let label = "x".repeat(63);
    let longest = [label.as_str(); 4].join(".");
    assert_eq!(longest.len(), 255);
    assert_eq!(
        refused(&format!("a@{longest}")),
        "the host name is too long"
    );
    let fits = &longest[..253];
    assert_eq!(parsed(&format!("a@{fits}")).1, fits);
    assert_eq!(parsed("a@192.0.2.10").1, "192.0.2.10");
}

#[test]
fn port_range() {
    let port = "the port must be a number from 1 to 65535";
    assert_eq!(parsed("a@mac:1").2, 1);
    assert_eq!(parsed("a@mac:65535").2, 65535);
    assert_eq!(refused("a@mac:0"), port);
    assert_eq!(refused("a@mac:65536"), port);
    assert_eq!(refused("a@mac:"), port);
    assert_eq!(refused("a@mac:+22"), port);
    assert_eq!(refused("a@mac:-1"), port);
    assert_eq!(refused("a@mac:22x"), port);
    assert_eq!(refused("a@mac:99999999999999999999"), port);
}

#[test]
fn commas_and_whitespace_are_refused() {
    assert_eq!(refused("a@mac,evil"), "the target contains a comma");
    assert_eq!(refused("a@mac.local,*"), "the target contains a comma");
    let space = "the target contains whitespace or control characters";
    assert_eq!(refused("a@mac local"), space);
    assert_eq!(refused(" a@mac"), space);
    assert_eq!(refused("a@mac\n"), space);
    assert_eq!(refused("a@mac\tx"), space);
    assert_eq!(refused("a@mac\u{0}"), space);
    assert_eq!(refused("a@mac\u{3000}"), space);
}

#[test]
fn ssh_urls_are_refused() {
    assert_eq!(
        refused("ssh://a@mac:22"),
        "write the target as user@host or user@host:port"
    );
}
