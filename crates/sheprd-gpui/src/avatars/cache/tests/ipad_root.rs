use super::super::ios_root;
use std::{ffi::OsString, path::PathBuf};

#[test]
fn the_ipad_cache_lives_in_the_containers_library_caches() {
    assert_eq!(
        ios_root(Some(OsString::from(
            "/private/var/mobile/Containers/Data/Application/A"
        ))),
        Some(PathBuf::from(
            "/private/var/mobile/Containers/Data/Application/A/Library/Caches/herdr-gpui/avatars-v1"
        ))
    );
}

#[test]
fn the_ipad_cache_needs_an_absolute_home() {
    assert_eq!(ios_root(None), None);
    assert_eq!(ios_root(Some(OsString::new())), None);
    assert_eq!(ios_root(Some(OsString::from("relative/home"))), None);
}
