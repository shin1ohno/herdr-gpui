use super::*;

#[test]
fn ipad_state_stays_in_the_container_application_support_directory() {
    let root = ios_state_dir_with(|name| match name {
        "HOME" => Some("/container".into()),
        "XDG_STATE_HOME" => Some("/outside".into()),
        _ => None,
    });
    assert_eq!(
        root,
        Some(PathBuf::from(
            "/container/Library/Application Support/herdr/gpui"
        ))
    );
}

#[test]
fn ipad_state_requires_a_nonempty_container_home() {
    assert_eq!(ios_state_dir_with(|_| None), None);
    assert_eq!(ios_state_dir_with(|_| Some("".into())), None);
}
