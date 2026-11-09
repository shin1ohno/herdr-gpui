use super::*;

#[gpui::test]
fn copy_as_one_line_follows_copy_and_has_a_noncolliding_binding(cx: &mut gpui::TestAppContext) {
    let model = menus(&Config::default());
    let edit = model
        .iter()
        .find(|menu| menu.name.as_ref() == "Edit")
        .unwrap();
    let copy = edit
        .items
        .iter()
        .position(|item| {
            matches!(item,
        MenuItem::Action { name, .. } if name.as_ref() == "Copy")
        })
        .unwrap();
    assert!(
        matches!(&edit.items[copy + 1], MenuItem::Action { name, action, os_action, .. }
        if name.as_ref() == "Copy as one line" && os_action.is_none()
        && action.partial_eq(&CopyAsOneLine))
    );
    let key = gpui::Keystroke::parse(crate::actions::COPY_AS_ONE_LINE_KEY).unwrap();
    let keymap = crate::keymap::Keymap::default();
    for (_, binding) in keymap.bindings() {
        let bound = crate::keymap::parse_keystroke(binding).unwrap();
        assert_ne!((&key.modifiers, &key.key), (&bound.modifiers, &bound.key));
    }
    cx.update(|cx| {
        crate::bind_keys(cx);
        let keys = cx.key_bindings();
        let keys = keys.borrow();
        let bindings: Vec<_> = keys.bindings_for_action(&CopyAsOneLine).collect();
        assert_eq!(bindings.len(), 1);
        assert_eq!(bindings[0].keystrokes()[0].inner(), &key);
    });
}
