use super::*;

#[test]
fn ipad_catalog_uses_application_support_and_ignores_external_state_roots() {
    for (development, app) in [(false, "herdr"), (true, "herdr-dev")] {
        assert_eq!(
            ios_catalog_path(development, |name| match name {
                "HOME" => Some("/container".into()),
                "XDG_STATE_HOME" => Some("/outside".into()),
                _ => None,
            }),
            PathBuf::from("/container/Library/Application Support")
                .join(app)
                .join("client/endpoints.json")
        );
    }
}
