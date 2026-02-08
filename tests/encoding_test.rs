use clip_frag::app::encoding::detect_encoding_and_decode;

#[test]
fn test_detect_utf8() {
    let data = "こんにちは".as_bytes().to_vec();

    let (decoded, enc) = detect_encoding_and_decode(&data).unwrap();

    assert_eq!(decoded, "こんにちは");
    assert_eq!(enc, "UTF-8");
}

#[test]
fn test_detect_shift_jis() {
    // "こんにちは" の Shift_JIS バイト列
    // 82 B1 82 F1 82 C9 82 BF 82 CD
    let sjis = vec![
        0x82, 0xB1, 0x82, 0xF1, 0x82, 0xC9, 0x82, 0xBF, 0x82, 0xCD,
    ];

    let (decoded, enc) = detect_encoding_and_decode(&sjis).unwrap();

    assert_eq!(decoded, "こんにちは");
    assert_eq!(enc, "Shift_JIS");
}

#[test]
fn test_invalid_encoding() {
    // UTF-8 としても Shift_JIS としても不正なバイト列
    let invalid = vec![0xFF, 0xFF, 0xFF];

    let result = detect_encoding_and_decode(&invalid);

    assert!(result.is_err());
}
