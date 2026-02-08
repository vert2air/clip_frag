use clip_frag::app::fragment::{
    build_fragment, calc_consumed_units, format_with_underscore,
};
use clip_frag::app::state::{AppState, Unit};

fn make_state(lines: Vec<&str>, max_unit: usize, unit: Unit) -> AppState {
    let lines: Vec<String> =
        lines.into_iter().map(|s| s.to_string()).collect();
    let line_units: Vec<usize> = lines
        .iter()
        .map(|s| match unit {
            Unit::Chars => s.chars().count(),
            Unit::Bytes => s.as_bytes().len(),
        })
        .collect();

    let total_units: usize = line_units.iter().sum();

    AppState {
        input_text: String::new(), // testでは不要なので、空でOK
        lines,
        line_units,
        total_units,
        max_unit,
        unit,
        prev_contents: String::new(),
        curr_index: 0,
        from_file: false, // testでは常にfalse
        input_file_name: None,
    }
}

#[test]
fn test_build_fragment_basic() {
    let state = make_state(vec!["aaa", "bbb", "ccc"], 4, Unit::Chars);

    let (frag, used, next) = build_fragment(&state, 0);

    assert_eq!(frag, "aaa");
    assert_eq!(used, 3);
    assert_eq!(next, 1);
}

#[test]
fn test_build_fragment_multi_line() {
    let state = make_state(vec!["12345", "67890", "abc"], 10, Unit::Chars);

    let (frag, used, next) = build_fragment(&state, 0);

    assert_eq!(frag, "12345\n67890".replace('\n', "")); // fragment は改行なし
    assert_eq!(used, 10);
    assert_eq!(next, 2);
}

#[test]
fn test_build_fragment_exact_fit() {
    let state = make_state(vec!["abcd", "efgh"], 8, Unit::Chars);

    let (frag, used, next) = build_fragment(&state, 0);

    assert_eq!(frag, "abcdefgh");
    assert_eq!(used, 8);
    assert_eq!(next, 2);
}

#[test]
fn test_build_fragment_exceeds() {
    let state = make_state(vec!["aaaa", "bbbb", "cccc"], 5, Unit::Chars);

    let (frag, used, next) = build_fragment(&state, 0);

    assert_eq!(frag, "aaaa");
    assert_eq!(used, 4);
    assert_eq!(next, 1);
}

#[test]
fn test_calc_consumed_units() {
    let state = make_state(vec!["aaa", "bb", "c"], 10, Unit::Chars);

    assert_eq!(calc_consumed_units(&state, 0), 0);
    assert_eq!(calc_consumed_units(&state, 1), 3);
    assert_eq!(calc_consumed_units(&state, 2), 5);
    assert_eq!(calc_consumed_units(&state, 3), 6);
}

#[test]
fn test_format_with_underscore() {
    assert_eq!(format_with_underscore(1), "1");
    assert_eq!(format_with_underscore(12), "12");
    assert_eq!(format_with_underscore(123), "123");
    assert_eq!(format_with_underscore(1234), "1_234");
    assert_eq!(format_with_underscore(12345), "12_345");
    assert_eq!(format_with_underscore(1234567), "1_234_567");
}
