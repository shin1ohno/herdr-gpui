use super::*;

#[test]
fn ipad_shortcut_table_and_prefix_chords_have_the_specified_glyphs() {
    for (input, expected) in [
        ("cmd-t", "⌘T"),
        ("ctrl-b c", "⌃B c"),
        ("cmd-shift-[", "⌘⇧["),
        ("cmd-shift-]", "⌘⇧]"),
        ("ctrl-b p", "⌃B p"),
        ("ctrl-b n", "⌃B n"),
        ("cmd-d", "⌘D"),
        ("ctrl-b v", "⌃B v"),
        ("cmd-shift-d", "⌘⇧D"),
        ("ctrl-b -", "⌃B -"),
        ("cmd-w", "⌘W"),
        ("ctrl-b x", "⌃B x"),
        ("cmd-shift-enter", "⌘⇧↩"),
        ("ctrl-b z", "⌃B z"),
        ("cmd-shift-c", "⌘⇧C"),
        ("ctrl-b [", "⌃B ["),
        ("cmd-p", "⌘P"),
        ("ctrl-b w", "⌃B w"),
        ("cmd-shift-n", "⌘⇧N"),
        ("ctrl-b shift-n", "⌃B ⇧N"),
        ("cmd-,", "⌘,"),
        ("ctrl-b s", "⌃B s"),
        ("cmd-/", "⌘/"),
        ("ctrl-b ?", "⌃B ?"),
        ("cmd-b", "⌘B"),
        ("ctrl-b b", "⌃B b"),
        ("cmd-shift-p", "⌘⇧P"),
    ] {
        assert_eq!(glyphs(input), expected, "{input}");
    }
    for number in 1..=9 {
        assert_eq!(glyphs(&format!("cmd-{number}")), format!("⌘{number}"));
        assert_eq!(glyphs(&format!("ctrl-b {number}")), format!("⌃B {number}"));
    }
}

#[test]
fn modifiers_named_keys_and_literal_punctuation_are_platform_independent() {
    for alias in ["cmd", "platform", "super", "win"] {
        assert_eq!(glyphs(&format!("shift-alt-ctrl-{alias}-p")), "⌘⌃⌥⇧P");
    }
    for (key, expected) in [
        ("enter", "↩"),
        ("escape", "⎋"),
        ("tab", "⇥"),
        ("backspace", "⌫"),
        ("left", "←"),
        ("up", "↑"),
        ("down", "↓"),
        ("right", "→"),
        ("space", "Space"),
        ("-", "-"),
        ("+", "+"),
    ] {
        assert_eq!(glyphs(&format!("cmd-{key}")), format!("⌘{expected}"));
    }
    assert_eq!(glyphs("cmd-shift--"), "⌘⇧-");
    assert_eq!(glyphs("ctrl-b alt-y"), "⌃B ⌥y");
    assert_eq!(glyphs("ctrl-b N"), "⌃B N");
    assert_eq!(glyphs(""), "");
}
