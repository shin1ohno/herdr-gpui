use super::*;

#[test]
fn shown_preserves_the_desktop_section_order() {
    assert_eq!(Section::shown().collect::<Vec<_>>(), Section::ALL);
    let _ipad = crate::ipad_ui::ipad_layout();
    let _desktop = crate::ipad_ui::desktop();
    assert_eq!(Section::shown().collect::<Vec<_>>(), Section::ALL);
}

#[test]
fn shown_hides_desktop_connection_sections_in_ipad_layout() {
    let _ipad = crate::ipad_ui::ipad_layout();
    let shown = Section::shown().collect::<Vec<_>>();
    assert!(shown.contains(&Section::General));
    assert!(!shown.contains(&Section::RemoteControl));
    assert!(!shown.contains(&Section::Plugins));
    #[cfg(feature = "cloud")]
    assert!(!shown.contains(&Section::CloudDevices));
    let mut expected = Section::ALL
        .iter()
        .copied()
        .filter(|section| match section {
            Section::Plugins | Section::RemoteControl => false,
            #[cfg(feature = "cloud")]
            Section::CloudDevices => false,
            _ => true,
        })
        .collect::<Vec<_>>();
    let tabs = expected
        .iter()
        .position(|section| *section == Section::Tabs)
        .unwrap();
    expected.insert(tabs + 1, Section::Keyboard);
    assert_eq!(shown, expected);
    let mut item = 0;
    for group in std::iter::once(None).chain(Group::ALL.map(Some)) {
        item += usize::from(group.is_some());
        for section in shown.iter().filter(|section| section.group() == group) {
            assert_eq!(section.nav_item(), item);
            item += 1;
        }
    }
}

#[gpui::test]
fn ipad_navigation_renders_only_shown_sections(cx: &mut TestAppContext) {
    let _ipad = crate::ipad_ui::ipad_layout();
    let (_, cx) = cx.add_window_view(skill_fixture);
    cx.simulate_resize(size(px(1000.), px(1200.)));
    cx.update(|window, cx| window.draw(cx).clear(cx));
    for section in Section::shown() {
        assert!(
            cx.debug_bounds(section.message()).is_some(),
            "missing section {section:?}"
        );
    }
    assert!(cx.debug_bounds(Section::RemoteControl.message()).is_none());
    assert!(cx.debug_bounds(Section::Plugins.message()).is_none());
    #[cfg(feature = "cloud")]
    assert!(cx.debug_bounds(Section::CloudDevices.message()).is_none());
}
