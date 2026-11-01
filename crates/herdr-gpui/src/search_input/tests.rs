use super::*;

mod completion;
mod composition;
mod control_keys;
mod keyboard_gesture;
mod placeholder;
mod rtl_caret;
mod word_keys;

#[test]
fn masked_offsets_count_characters_both_ways() {
    let text = "a\u{e9}\u{1f600}z";
    for (shown, (byte, _)) in text.char_indices().enumerate() {
        assert_eq!(masked_offset(text, byte), shown);
        assert_eq!(unmasked_offset(text, shown), byte);
    }
    assert_eq!(masked_offset(text, text.len()), 4);
    assert_eq!(unmasked_offset(text, 4), text.len());
    assert_eq!(unmasked_offset(text, 99), text.len());
}

#[test]
fn utf16_ranges_are_clamped_to_scalar_boundaries() {
    let text = "a\u{1f600}\u{e9}\u{4e2d}";
    assert_eq!(byte_range(text, 1..3), 1..5);
    assert_eq!(byte_range(text, 2..99), 5..text.len());
    assert_eq!(byte_range(text, Range { start: 99, end: 1 }), 1..text.len());
    for (byte, _) in text.char_indices() {
        assert_eq!(from_utf16(text, to_utf16(text, byte)), byte);
    }
}

#[test]
fn replaces_unicode_selection_and_explicit_utf16_range() {
    let mut edit = Editing::default();
    assert!(edit.replace(None, "a\u{1f600}\u{e9}z", false, None));
    edit.anchor = 7;
    edit.cursor = 1;
    assert_eq!(edit.selection(), 1..7);
    assert!(edit.replace(None, "\u{4e2d}", false, None));
    assert_eq!(edit.text, "a\u{4e2d}z");
    assert_eq!(edit.cursor, 4);
    assert!(edit.replace(Some(1..2), "\u{1f600}", false, None));
    assert_eq!(edit.text, "a\u{1f600}z");
    assert_eq!(edit.cursor, 5);
    assert_eq!(edit.previous(), 1);
    edit.select_to(1, false);
    assert_eq!(edit.next(), 5);
}

#[test]
fn composition_selection_is_relative_to_inserted_text() {
    let mut edit = Editing::default();
    edit.replace(None, "\u{1f600}prefix", false, None);
    let start = edit.cursor;
    edit.replace(None, "\u{4e2d}\u{1f600}a", true, Some(1..3));
    assert_eq!(edit.selection(), start + 3..start + 7);
    assert_eq!(edit.marked, Some(start..start + 8));
    edit.replace(None, "\u{e9}", true, Some(0..1));
    assert_eq!(edit.selection(), start..start + 2);
    assert_eq!(edit.marked, Some(start..start + 2));
    assert!(!edit.replace(None, "\u{e9}", false, None));
    assert_eq!(edit.marked, None);
    assert_eq!(edit.selection(), start + 2..start + 2);
    assert_eq!(edit.text, "\u{1f600}prefix\u{e9}");
}

#[test]
fn strips_newlines_and_maps_composition_selection() {
    let mut edit = Editing::default();
    edit.replace(None, "a\r\n\u{1f600}\nz", true, Some(3..6));
    assert_eq!(edit.text, "a\u{1f600}z");
    assert_eq!(edit.selection(), 1..5);
    edit.replace(None, "", true, None);
    assert!(edit.text.is_empty());
    assert_eq!(edit.marked, None);
    assert_eq!(edit.selection(), 0..0);
    assert!(!edit.replace(None, "\r\n", false, None));
}

#[gpui::test]
fn cmd_arrows_and_cmd_backspace_reach_the_line_ends(cx: &mut gpui::TestAppContext) {
    let (input, cx) = cx.add_window_view(|_, cx| SearchInput::new(cx));
    cx.update(|window, cx| {
        input.update(cx, |input, cx| {
            input.edit.replace(None, "one two", false, None);
            input.edit.select_to(3, false);
            window.focus(&input.focus, cx);
        });
        window.draw(cx).clear(cx);
    });
    let edit = |cx: &mut gpui::VisualTestContext| {
        input.read_with(cx, |input, _| {
            (input.edit.text.clone(), input.edit.selection())
        })
    };
    cx.simulate_keystrokes("cmd-shift-right");
    assert_eq!(edit(cx), ("one two".into(), 3..7));
    cx.simulate_keystrokes("cmd-left");
    assert_eq!(edit(cx), ("one two".into(), 0..0));
    cx.simulate_keystrokes("cmd-right left left cmd-backspace");
    assert_eq!(edit(cx), ("wo".into(), 0..0));
    cx.simulate_keystrokes("cmd-delete");
    assert_eq!(edit(cx), (String::new(), 0..0));
}

#[test]
fn extending_selection_can_cross_its_anchor() {
    let mut edit = Editing::default();
    edit.replace(None, "a\u{1f600}b", false, None);
    edit.select_to(1, false);
    edit.select_to(0, true);
    assert_eq!(edit.selection(), 0..1);
    edit.select_to(5, true);
    assert_eq!(edit.selection(), 1..5);
    edit.select_to(edit.next(), true);
    assert_eq!(edit.selection(), 1..6);
}
