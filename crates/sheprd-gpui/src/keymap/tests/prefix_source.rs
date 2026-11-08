use super::*;

#[test]
fn parsed_provenance_distinguishes_explicit_default_from_fallback() {
    for (text, configured, expected) in [
        ("", false, "ctrl-b"),
        ("[keys]\nprefix = 'ctrl+b'", true, "ctrl-b"),
        ("[keys]\nprefix = 'ctrl+a'", true, "ctrl-a"),
        ("[keys]\nprefix = []", false, "ctrl-b"),
        ("[keys]\nprefix = 5", false, "ctrl-b"),
        ("[keys]\nprefix = 'hyper+invalid'", false, "ctrl-b"),
        (
            "[keys]\nprefix = []\nextra_prefixes = ['ctrl+a']",
            true,
            "ctrl-a",
        ),
    ] {
        let table: toml::Table = text.parse().unwrap();
        let keys = DaemonKeys::from_table(table.get("keys").and_then(toml::Value::as_table));
        let map = Keymap::with_overrides(&BTreeMap::new(), &PaneKeys::new(), &keys).unwrap();
        assert_eq!(
            matches!(keys.prefix_source(), LocalPrefixSource::Configured(_)),
            configured,
            "{text}"
        );
        assert_eq!(map.prefix_label().as_deref(), Some(expected), "{text}");
    }
}
