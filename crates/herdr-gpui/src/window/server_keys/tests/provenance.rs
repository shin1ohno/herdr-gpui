use super::*;
use crate::keymap::{KeymapSource, LocalPrefixSource};
use std::path::Path;

#[gpui::test]
fn resolved_value_and_origin_follow_ack_fallback_and_host_switch(cx: &mut TestAppContext) {
    let (view, cx) = window(cx);
    view.update(cx, |view, cx| {
        let table: toml::Table = "[keys]\nprefix='ctrl+a'".parse().unwrap();
        let mut keys = DaemonKeys::from_table(table.get("keys").and_then(toml::Value::as_table));
        keys.set_config_path(Path::new("/private-fixture/local-herdr.toml"));
        view.config.keybinding_prefix_source = keys.prefix_source().clone();
        view.config.keybindings = Keymap::with_overrides(&BTreeMap::new(), &PaneKeys::new(), &keys).unwrap();
        opt_in(view, KeybindingSource::Server);
        for profile in [None, Some("[keys"), Some("prefix='ctrl+b'")] {
            select(view, 1, profile, cx);
            let resolved = view.resolved_keymap();
            assert!(std::ptr::eq(resolved.keymap, view.keymap()));
            assert!(resolved.keymap.is_prefix(&key("ctrl-a")));
            assert!(matches!(resolved.source, KeymapSource::Local(LocalPrefixSource::Configured(Some(path)))
                if path == Path::new("/private-fixture/local-herdr.toml")));
        }
        select(view, 1, Some(PROFILE), cx);
        let resolved = view.resolved_keymap();
        assert!(resolved.keymap.is_prefix(&key("cmd-j")));
        assert!(matches!(resolved.source, KeymapSource::Server("box")));
        // A valid normalized profile containing the default key still comes
        // from that host; equality to Ctrl-B cannot establish remote origin.
        select(view, 1, Some("[keys]\nprefix='ctrl+b'"), cx);
        assert!(matches!(view.resolved_keymap().source, KeymapSource::Server("box")));
        // Before the normal selection synchronization runs, input still uses
        // the cached server map. Its displayed origin must follow that map.
        view.selected_endpoint = 0;
        assert!(view.keymap().is_prefix(&key("ctrl-b")));
        assert!(matches!(view.resolved_keymap().source, KeymapSource::Server("box")));
        select(view, 0, None, cx);
        assert!(matches!(view.resolved_keymap().source, KeymapSource::Local(_)));
        assert!(view.keymap().is_prefix(&key("ctrl-a")));
        select(view, 1, Some(PROFILE), cx);
        opt_in(view, KeybindingSource::Local);
        view.sync_server_keymap(cx);
        assert!(matches!(view.resolved_keymap().source, KeymapSource::Local(_)));
        view.config.keybindings = Keymap::default();
        view.config.keybinding_prefix_source = LocalPrefixSource::Default(None);
        assert!(matches!(view.resolved_keymap().source, KeymapSource::Local(LocalPrefixSource::Default(None))));
    });
}

#[gpui::test]
fn provenance_only_profile_change_preserves_an_armed_chord(cx: &mut TestAppContext) {
    let (view, cx) = window(cx);
    view.update(cx, |view, cx| {
        opt_in(view, KeybindingSource::Server);
        select(view, 1, Some("[keys]\nprefix='ctrl+b'"), cx);
    });
    cx.update(|window, cx| {
        window.focus(&view.read(cx).focus.clone(), cx);
        window.draw(cx).clear(cx);
        assert!(window.dispatch_keystroke(key("ctrl-b"), cx));
    });
    cx.run_until_parked();
    view.update(cx, |view, cx| {
        assert!(view.prefix_armed);
        let before = view.keymap().clone();
        // The accepted profile still binds Ctrl-B; only parser provenance
        // changes from explicitly configured to fallback.
        select(view, 1, Some("[keys]"), cx);
        assert_eq!(*view.keymap(), before);
        assert!(view.prefix_armed, "provenance must not cancel a chord");
        assert!(matches!(
            view.resolved_keymap().source,
            KeymapSource::Server("box")
        ));
        select(view, 1, Some(PROFILE), cx);
        assert!(!view.prefix_armed, "a genuinely changed prefix must disarm");
        assert!(view.keymap().is_prefix(&key("cmd-j")));
    });
}
