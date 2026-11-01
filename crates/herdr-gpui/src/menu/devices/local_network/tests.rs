use super::settings_url;

#[test]
fn permission_settings_open_on_the_current_platform() {
    if cfg!(target_os = "ios") {
        assert_eq!(settings_url(), Some("app-settings:"));
    } else if cfg!(target_os = "macos") {
        assert_eq!(
            settings_url(),
            Some("x-apple.systempreferences:com.apple.preference.security?Privacy_LocalNetwork")
        );
    } else {
        assert_eq!(settings_url(), None);
    }
}
