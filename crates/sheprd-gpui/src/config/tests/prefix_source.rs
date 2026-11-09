use super::*;
use crate::keymap::LocalPrefixSource;

#[test]
fn loaded_keymap_keeps_the_exact_local_path_even_when_defaults_apply() -> anyhow::Result<()> {
    let temp = TempDirectory::new()?;
    let gui = temp.0.join("config-gpui.toml");
    let daemon = temp.0.join("custom-daemon.toml");
    fs::write(&gui, "")?;
    let load = || Config::load_path(&gui, &daemon);
    assert_eq!(
        load()?.keybinding_prefix_source,
        LocalPrefixSource::Default(Some(daemon.clone()))
    );
    for (text, explicit) in [
        ("[keys]\nprefix='ctrl+b'", true),
        ("[keys]\nprefix='ctrl+a'", true),
        ("[keys]\nprefix=[]", false),
        ("[keys]\nprefix='hyper+invalid'", false),
        ("[keys", false),
    ] {
        fs::write(&daemon, text)?;
        let config = load()?;
        assert_eq!(
            config.keybinding_prefix_source.path(),
            Some(daemon.as_path())
        );
        assert_eq!(
            matches!(
                config.keybinding_prefix_source,
                LocalPrefixSource::Configured(_)
            ),
            explicit,
            "{text}"
        );
    }
    fs::write(&daemon, " ".repeat((MAX_DAEMON_CONFIG_BYTES + 1) as usize))?;
    assert_eq!(
        load()?.keybinding_prefix_source,
        LocalPrefixSource::Default(Some(daemon))
    );
    Ok(())
}
