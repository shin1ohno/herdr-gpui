use super::*;

#[gpui::test]
fn ipad_group_menu_hides_browser_creation_without_changing_desktop(cx: &mut TestAppContext) {
    let (view, cx) = window(cx);
    for ipad in [false, true] {
        let _ipad = ipad.then(crate::ipad_ui::ipad_layout);
        let group = groups(&view, cx)[0];
        let offered = actions(&view, cx, group);
        assert_eq!(offered.contains(&Action::NewBrowserTab), !ipad);
        assert!(offered.contains(&Action::Split));
    }
}
