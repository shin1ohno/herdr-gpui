use super::*;

#[test]
fn echoed_identifiers_are_short_and_printable() {
    assert!(plain("w_1", 256));
    for invalid in ["", "w 1", "w\u{1b}[2J", "w\n1"] {
        assert!(!plain(invalid, 256), "{invalid:?}");
    }
    assert!(!plain(&"w".repeat(257), 256));
}

#[test]
fn daemon_paths_may_hold_spaces_but_not_control_characters() {
    assert!(plain_path(
        r"C:\Users\Jane Doe\AppData\Roaming\herdr\herdr-client.sock"
    ));
    assert!(plain_path("/home/jane/.config/herdr/herdr.sock"));
    for invalid in ["", "/tmp/a\nb.sock", "/tmp/\u{1b}[2J"] {
        assert!(!plain_path(invalid), "{invalid:?}");
    }
    assert!(!plain_path(&"a".repeat(4097)));
}
