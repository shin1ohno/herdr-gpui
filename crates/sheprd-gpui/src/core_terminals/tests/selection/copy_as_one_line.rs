use super::*;

#[test]
fn joining_preserves_normal_copy_and_padding() {
    for (rows, normal, joined) in [
        (
            ["https://ex", "abcdefghij", "0123456789"],
            "https://ex\nabcdefghij\n0123456789",
            "https://exabcdefghij0123456789",
        ),
        (
            ["short", "next", "last"],
            "short\nnext\nlast",
            "short\nnext\nlast",
        ),
        (
            ["abcdefghij", "", "end"],
            "abcdefghij\n\nend",
            "abcdefghij\nend",
        ),
    ] {
        let shown = frame(10, &rows);
        let mut selection = Selection::at(at(0, 0));
        selection.extend(at(2, 9));
        assert_eq!(selection.text(&shown), normal);
        assert_eq!(selection.text_with_join(&shown), joined);
    }
}

#[test]
fn wide_edge_and_multibyte_symbols_join_without_duplication() {
    let mut shown = frame(10, &["abcdefgh界 ", "next", "last"]);
    shown.cells[9].skip = true;
    shown.cells[9].symbol.clear();
    let mut selection = Selection::at(at(0, 0));
    selection.extend(at(2, 9));
    assert_eq!(selection.text(&shown), "abcdefgh界\nnext\nlast");
    assert_eq!(selection.text_with_join(&shown), "abcdefgh界next\nlast");
    shown.cells[8].symbol = "é".into();
    shown.cells[9].skip = false;
    shown.cells[9].symbol = "é".into();
    assert_eq!(selection.text_with_join(&shown), "abcdefghéénext\nlast");
}
