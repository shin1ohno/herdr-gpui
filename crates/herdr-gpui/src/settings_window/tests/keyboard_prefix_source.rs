use super::*;
use crate::{
    keymap::{KeymapSource, LocalPrefixSource},
    settings_window::pages::keyboard::prefix_guidance,
};
use std::path::PathBuf;

#[test]
fn guidance_names_the_applied_source_and_the_edit_location_separately_from_style() {
    crate::i18n::show(&herdr_i18n::Locale::FALLBACK);
    let local = LocalPrefixSource::Configured(Some(PathBuf::from("/fixture/leader.toml")));
    let (origin, edit) = prefix_guidance(KeymapSource::Local(&local));
    assert!(origin.contains("local Herdr config"));
    assert!(edit.contains("/fixture/leader.toml") && edit.contains("[keys].prefix"));
    assert!(edit.contains("does not edit the leader"));
    let default = LocalPrefixSource::Default(Some(PathBuf::from("/fixture/missing.toml")));
    let (origin, edit) = prefix_guidance(KeymapSource::Local(&default));
    assert!(origin.contains("Herdr's default"));
    assert!(edit.contains("/fixture/missing.toml"));
    let (origin, edit) = prefix_guidance(KeymapSource::Server("studio-mac"));
    assert!(origin.contains("studio-mac"));
    assert!(edit.contains("on that host") && edit.contains("does not edit remote config"));
}
