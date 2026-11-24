use super::*;
use sheprd_protocol::{MAX_FRAME_SIZE, decode_payload, encode_message};

/// Every cell carrying `SOFT_WRAP`, as `(x, y)`.
fn marks(frame: &FrameData) -> Vec<(u16, u16)> {
    (0..frame.height)
        .flat_map(|y| (0..frame.width).map(move |x| (x, y)))
        .filter(|&(x, y)| cell(frame, x, y).modifier & SOFT_WRAP != 0)
        .collect()
}

/// The non-blank rows from the top of history down, each with whether its
/// last cell carries the mark.
fn rows_from_the_top(emulator: &mut Emulator) -> Vec<(String, bool)> {
    emulator.scroll(Scroll::Top);
    let frame = emulator.frame();
    let last = frame.width - 1;
    let rows = (0..frame.height)
        .map(|y| {
            (
                line(&frame, y),
                cell(&frame, last, y).modifier & SOFT_WRAP != 0,
            )
        })
        .filter(|(text, _)| !text.is_empty())
        .collect();
    emulator.scroll(Scroll::Bottom);
    rows
}

fn settled(cols: u16, rows: u16, bytes: &[u8]) -> Emulator {
    let mut emulator = fed(cols, rows, bytes);
    assert!(matches!(emulator.take_damage(), Damage::Full(_)));
    emulator
}

#[test]
fn a_row_the_terminal_wraps_marks_its_last_cell() {
    let frame = fed(20, 3, &[b'a'; 25]).frame();
    assert_eq!(line(&frame, 0), "a".repeat(20));
    assert_eq!(line(&frame, 1), "a".repeat(5));
    assert_eq!(marks(&frame), [(19, 0)]);
}

#[test]
fn a_line_ending_at_the_edge_is_not_marked() {
    let mut bytes = vec![b'a'; 20];
    bytes.extend_from_slice(b"\r\nnext");
    let frame = fed(20, 3, &bytes).frame();
    assert_eq!(line(&frame, 0), "a".repeat(20));
    assert_eq!(line(&frame, 1), "next");
    assert_eq!(marks(&frame), []);
    // A full row with nothing after it has not wrapped yet: alacritty only
    // notes that the next character must wrap.
    let pending = fed(20, 3, &[b'a'; 20]).frame();
    assert_eq!(marks(&pending), []);
}

#[test]
fn a_wide_glyph_that_does_not_fit_marks_the_leading_spacer() {
    let frame = fed(5, 2, "abcd日".as_bytes()).frame();
    let spacer = cell(&frame, 4, 0);
    assert_eq!(spacer.symbol, " ");
    assert!(!spacer.skip);
    assert_eq!(marks(&frame), [(4, 0)]);
    assert_eq!(cell(&frame, 0, 1).symbol, "日");
    assert!(cell(&frame, 1, 1).skip);
}

#[test]
fn a_wide_glyph_ending_the_row_marks_its_spacer_cell() {
    // Alacritty flags the cell the cursor wrapped from, which is the second
    // half of a wide glyph that filled the row.
    let frame = fed(5, 2, "abc日e".as_bytes()).frame();
    assert_eq!(cell(&frame, 3, 0).symbol, "日");
    assert!(cell(&frame, 4, 0).skip);
    assert_eq!(marks(&frame), [(4, 0)]);
    assert_eq!(line(&frame, 1), "e");
}

#[test]
fn reflow_keeps_the_marks_with_the_rows() {
    let mut bytes = vec![b'a'; 15];
    bytes.extend_from_slice(b"\r\nnext");
    let mut emulator = fed(10, 6, &bytes);
    assert_eq!(
        rows_from_the_top(&mut emulator),
        [
            ("a".repeat(10), true),
            ("a".repeat(5), false),
            ("next".to_owned(), false),
        ]
    );
    emulator.resize(size(5, 6));
    assert_eq!(
        rows_from_the_top(&mut emulator),
        [
            ("a".repeat(5), true),
            ("a".repeat(5), true),
            ("a".repeat(5), false),
            ("next".to_owned(), false),
        ]
    );
    assert!(marks(&emulator.frame()).iter().all(|&(x, _)| x == 4));
    emulator.resize(size(10, 6));
    assert_eq!(
        rows_from_the_top(&mut emulator),
        [
            ("a".repeat(10), true),
            ("a".repeat(5), false),
            ("next".to_owned(), false),
        ]
    );
    assert!(marks(&emulator.frame()).iter().all(|&(x, _)| x == 9));
}

#[test]
fn patched_rows_carry_the_mark() {
    let mut emulator = settled(10, 3, b"");
    emulator.feed(&[b'a'; 15]);
    let Damage::Rows { rows, .. } = emulator.take_damage() else {
        panic!("expected row damage");
    };
    let mark = |y: u16| {
        rows.iter()
            .find(|row| row.y == y)
            .map(|row| row.cells[9].modifier & SOFT_WRAP != 0)
    };
    assert_eq!(mark(0), Some(true));
    assert_eq!(mark(1), Some(false));
}

#[test]
fn a_scrolled_view_keeps_the_marks() {
    let mut emulator = Emulator::new(size(10, 3), 100);
    let mut bytes = vec![b'a'; 15];
    for line in 0..6 {
        bytes.extend_from_slice(format!("\r\n{line}").as_bytes());
    }
    emulator.feed(&bytes);
    assert!(matches!(emulator.take_damage(), Damage::Full(_)));
    assert_eq!(marks(&emulator.frame()), []);
    emulator.scroll(Scroll::Top);
    assert!(emulator.scroll_metrics().offset > 0);
    let frame = emulator.frame();
    assert_eq!(line(&frame, 0), "a".repeat(10));
    assert_eq!(line(&frame, 1), "a".repeat(5));
    assert_eq!(marks(&frame), [(9, 0)]);
    let Damage::Full(full) = emulator.take_damage() else {
        panic!("expected full damage after a scroll");
    };
    assert_eq!(marks(&full), [(9, 0)]);
    emulator.scroll(Scroll::Lines(-1));
    assert_eq!(marks(&emulator.frame()), []);
}

#[test]
fn the_mark_is_bit_nine_and_crosses_the_wire_unchanged() {
    assert_eq!(SOFT_WRAP, 1 << 9);
    assert_eq!(
        SOFT_WRAP & (BOLD | DIM | ITALIC | UNDERLINE | REVERSED | HIDDEN | STRIKETHROUGH),
        0
    );
    let frame = fed(20, 3, &[b'a'; 25]).frame();
    let marked = cell(&frame, 19, 0).clone();
    assert_eq!(marked.modifier, SOFT_WRAP);
    let bytes = encode_message(&marked, MAX_FRAME_SIZE).unwrap();
    let decoded: CellData = decode_payload(&bytes[4..]).unwrap();
    assert_eq!(decoded, marked);
}
