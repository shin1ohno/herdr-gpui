use crate::input::terminal_text_focus;

#[test]
fn the_desktop_drops_text_focus_while_a_pane_waits_on_a_resize() {
    // focused, not in copy mode, waiting on the same target, not yet ready
    assert!(!terminal_text_focus(false, true, false, true, false));
}

#[test]
fn the_ipad_keeps_text_focus_while_a_pane_waits_on_a_resize() {
    assert!(terminal_text_focus(true, true, false, true, false));
}

#[test]
fn both_policies_agree_outside_a_resize_wait() {
    for retain in [false, true] {
        // a ready pane takes text, focused or not (focus is checked inside)
        assert!(terminal_text_focus(retain, true, false, false, true));
        // copy mode keeps its keys while focused
        assert!(terminal_text_focus(retain, true, true, false, false));
        // nothing focused, nothing ready
        assert!(!terminal_text_focus(retain, false, false, true, false));
        assert!(!terminal_text_focus(retain, true, false, false, false));
    }
}
