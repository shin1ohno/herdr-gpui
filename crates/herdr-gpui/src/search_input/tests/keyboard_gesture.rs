use super::*;

#[gpui::test]
fn repeated_field_taps_keep_text_input_and_composition(cx: &mut gpui::TestAppContext) {
    let (input, cx) = cx.add_window_view(|_, cx| SearchInput::new(cx));
    cx.update(|window, cx| window.draw(cx).clear(cx));
    let bounds = input
        .read_with(cx, |input, _| input.bounds)
        .unwrap_or_default();

    cx.simulate_click(bounds.center(), gpui::Modifiers::none());
    assert!(cx.update(|window, cx| input.read(cx).focus.is_focused(window)));
    cx.simulate_input("draft");

    // UIKit can hide its keyboard while GPUI focus stays on this field.
    cx.update(|window, _| window.dismiss_virtual_keyboard());
    cx.simulate_click(bounds.center(), gpui::Modifiers::none());
    assert!(cx.update(|window, cx| input.read(cx).focus.is_focused(window)));
    cx.simulate_keystrokes("end");
    cx.simulate_input(" text");
    assert_eq!(
        input.read_with(cx, |input, _| input.text().to_owned()),
        "draft text"
    );

    cx.update(|window, cx| {
        input.update(cx, |input, cx| {
            input.replace_and_mark_text_in_range(None, "中文", Some(0..2), window, cx);
        });
        window.draw(cx).clear(cx);
    });
    let before = input.read_with(cx, |input, _| {
        (
            input.edit.text.clone(),
            input.edit.selection(),
            input.edit.marked.clone(),
        )
    });
    cx.update(|window, _| window.dismiss_virtual_keyboard());
    cx.simulate_click(bounds.center(), gpui::Modifiers::none());
    assert_eq!(
        input.read_with(cx, |input, _| {
            (
                input.edit.text.clone(),
                input.edit.selection(),
                input.edit.marked.clone(),
            )
        }),
        before,
    );
}
