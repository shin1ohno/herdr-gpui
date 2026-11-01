use super::*;

#[test]
fn ipad_config_stays_in_application_support_and_ignores_external_roots() {
    assert_eq!(
        ios_config_root_with(|name| match name {
            "HOME" => Some("/container".into()),
            "XDG_CONFIG_HOME" => Some("/outside".into()),
            _ => None,
        }),
        Ok(PathBuf::from("/container/Library/Application Support"))
    );
}

#[test]
fn ipad_config_requires_a_nonempty_container_home() {
    assert_eq!(ios_config_root_with(|_| None), Err(PathError::MissingHome));
    assert_eq!(
        ios_config_root_with(|_| Some("".into())),
        Err(PathError::MissingHome)
    );
}
