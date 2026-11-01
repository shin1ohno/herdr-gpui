use super::*;
use crate::config::ShortcutStyle::{HerdrPrefix, Ipad};

#[test]
fn styles_select_resolved_direct_bindings_or_chords() {
    let keymap = Keymap::default();
    assert_eq!(keymap.styled(Command::Tab, Ipad), Some(default("cmd-t")));
    assert_eq!(keymap.styled(Command::Tab, HerdrPrefix), Some("ctrl-b c"));
    assert_eq!(keymap.styled(Command::Palette, HerdrPrefix), None);
    assert_eq!(keymap.styled(Command::Themes, Ipad), None);
    let changed = Keymap::with_overrides(
        &overrides(&[("new_tab", one("cmd-t"))]),
        &Default::default(),
        &DaemonKeys::default(),
    )
    .unwrap();
    assert_eq!(changed.styled(Command::Tab, Ipad), Some("cmd-t"));
    assert_eq!(changed.styled(Command::Tab, HerdrPrefix), None);
}

#[test]
fn styles_follow_the_hosts_prefix_and_first_configured_alias() {
    let table = "prefix = 'ctrl+a'\nnew_workspace = ''\nnew_tab = 'prefix+shift+n'"
        .parse()
        .unwrap();
    let keymap = Keymap::with_overrides(
        &BTreeMap::new(),
        &Default::default(),
        &DaemonKeys::from_table(Some(&table)),
    )
    .unwrap();
    assert_eq!(
        keymap.styled(Command::Tab, HerdrPrefix),
        Some("ctrl-a shift-n")
    );
    let keymap = with(&[(
        "new_tab",
        Binding::Many(vec!["alt-t".into(), "cmd-t".into()]),
    )])
    .unwrap();
    assert_eq!(keymap.styled(Command::Tab, Ipad), Some("alt-t"));
}
