#![allow(clippy::unwrap_used)]

/// Desktop platforms need explicit text/backend features; the iOS platform
/// provides its own text system. Inspecting the manifest needs no native window.
#[test]
fn the_platform_crate_keeps_its_text_features() {
    let manifest: toml::Table = toml::from_str(include_str!("../../Cargo.toml")).unwrap();
    let desktop = &manifest["target"]["cfg(not(target_os = \"ios\"))"]["dependencies"];
    let features = desktop["gpui_platform"]["features"].as_array().unwrap();
    for feature in ["font-kit", "wayland", "x11"] {
        assert!(
            features.iter().any(|value| value.as_str() == Some(feature)),
            "gpui_platform is missing the {feature} feature"
        );
    }
    assert!(
        !manifest["dependencies"]
            .as_table()
            .unwrap()
            .contains_key("gpui_platform")
    );
    let ios = &manifest["target"]["cfg(target_os = \"ios\")"]["dependencies"];
    assert!(ios.as_table().unwrap().contains_key("herdr-uikit"));
    assert!(!ios.as_table().unwrap().contains_key("gpui_platform"));
}
