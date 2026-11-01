//! The desktop SSH host-key action remains available with remote control off.

use super::*;

#[test]
fn host_key_code_catalog_row_is_found_by_ssh_and_qr_searches() {
    let entry = catalog::CATALOG
        .iter()
        .find(|entry| entry.id == "remote-host-key-code")
        .unwrap();
    assert_eq!(entry.section, Section::RemoteControl);
    for query in ["SSH", "QR code", "host-key fingerprints"] {
        assert!(
            catalog::search(query)
                .iter()
                .any(|index| catalog::CATALOG[*index].id == entry.id),
            "{query}"
        );
    }
}

#[gpui::test]
fn host_key_code_row_is_shown_when_the_remote_control_service_is_off(cx: &mut TestAppContext) {
    let (view, cx) = cx.add_window_view(skill_fixture);
    view.update(cx, |view, _| {
        view.section = Section::RemoteControl;
        view.config.remote.enabled = false;
    });
    cx.simulate_resize(size(px(960.), px(3000.)));
    cx.update(|window, cx| crate::sidebar::layout_tests::full_draw(window, cx).clear(cx));
    assert!(cx.debug_bounds("remote-host-key-code").is_some());
    assert!(cx.debug_bounds("remote-host-key-code-show").is_some());
}
