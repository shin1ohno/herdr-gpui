use super::*;

#[test]
fn joining_preserves_normal_copy_and_padding() {
    for (rows, normal, joined) in [
        (
            "https://exabcdefghij0123456789",
            "https://ex\nabcdefghij\n0123456789",
            "https://exabcdefghij0123456789",
        ),
        (
            "short     next      last      ",
            "short\nnext\nlast",
            "short\nnext\nlast",
        ),
        (
            "abcdefghij          end       ",
            "abcdefghij\n\nend",
            "abcdefghij\nend",
        ),
    ] {
        let surface = surface(rows);
        let selection = drag(&surface, (0., 10.), (100., 50.));
        assert_eq!(text(&surface, &selection), normal);
        assert_eq!(
            selection
                .text_with_join(&surface, CELL_WIDTH, CELL_HEIGHT, true)
                .unwrap(),
            joined
        );
        assert_eq!(
            selection
                .text_with_join(&surface, CELL_WIDTH, CELL_HEIGHT, false)
                .unwrap(),
            normal
        );
    }
}

#[test]
fn wide_edge_and_multibyte_symbols_join_without_duplication() {
    let mut surface = surface("abcdefgh界 next      last      ");
    surface.frame.cells[9].symbol = String::new();
    let selection = drag(&surface, (0., 10.), (100., 50.));
    assert_eq!(text(&surface, &selection), "abcdefgh界\nnext\nlast");
    assert_eq!(
        selection
            .text_with_join(&surface, CELL_WIDTH, CELL_HEIGHT, true)
            .unwrap(),
        "abcdefgh界next\nlast"
    );
    surface.frame.cells[8].symbol = "é".into();
    surface.frame.cells[9].symbol = "é".into();
    assert_eq!(
        selection
            .text_with_join(&surface, CELL_WIDTH, CELL_HEIGHT, true)
            .unwrap(),
        "abcdefghéénext\nlast"
    );
}
