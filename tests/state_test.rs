use clip_frag::app::state::{AppState, Unit};

#[test]
fn test_split_lines_preserve_newline() {
    let text = "aaa\nbbb\r\nccc";
    let st = AppState::new(
        text.to_string(),
        Unit::Chars,
        100,
        false,
        None,
    );

    assert_eq!(st.lines.len(), 3);
    assert_eq!(st.lines[0], "aaa\n");
    assert_eq!(st.lines[1], "bbb\r\n");
    assert_eq!(st.lines[2], "ccc");
}

#[test]
fn test_line_units_chars() {
    let text = "あ\nいい\n";
    let st = AppState::new(
        text.to_string(),
        Unit::Chars,
        100,
        false,
        None,
    );

    // "あ\n" → 2 chars
    // "いい\n" → 3 chars
    assert_eq!(st.line_units, vec![2, 3]);
    assert_eq!(st.total_units, 5);
}

#[test]
fn test_line_units_bytes() {
    let text = "あ\nい\n";
    let st = AppState::new(
        text.to_string(),
        Unit::Bytes,
        100,
        false,
        None,
    );

    // UTF-8: "あ" = 3 bytes, "\n" = 1 byte
    assert_eq!(st.line_units, vec![4, 4]);
    assert_eq!(st.total_units, 8);
}

#[test]
fn test_state_metadata() {
    let st = AppState::new(
        "abc".to_string(),
        Unit::Chars,
        10,
        true,
        Some("file.txt".to_string()),
    );

    assert!(st.from_file);
    assert_eq!(st.input_file_name.as_deref(), Some("file.txt"));
    assert_eq!(st.curr_index, 0);
    assert_eq!(st.prev_contents, "");
}
