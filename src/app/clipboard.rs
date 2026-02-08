// ============================================================================
// src/app/clipboard.rs
// ============================================================================
//
// このファイルでは、Windows のクリップボード操作を安全に扱うための
// ラッパー関数を提供する。
//
// clip_frag が依存する clipboard-win v5.4 の API は、
//   set_clipboard(formats::Unicode, text) -> Result<(), ErrorCode>
// のように、ErrorCode を返す。
//
// しかし ErrorCode は std::error::Error を実装していないため、
// anyhow::Error に自動変換できず、`?` 演算子が使えない。
//
// そこで本モジュールでは、ErrorCode を anyhow::Error に変換する
// set_clip_utf16() を提供し、アプリ本体からは安全に `?` が使えるようにする。
//
// また、クリップボードをクリアする clear_clipboard() も提供する。
// ============================================================================

use anyhow::Result;
use clipboard_win::{formats, set_clipboard};

// -----------------------------------------------------------------------------
// set_clip_utf16
// -----------------------------------------------------------------------------
//
// clipboard-win v5.4 の set_clipboard() を安全にラップする関数。
//
// - Unicode 文字列をクリップボードに設定する。
// - ErrorCode を anyhow::Error に変換する。
// - アプリ本体では set_clip_utf16(text)?; と書くだけでよい。
// -----------------------------------------------------------------------------

//pub fn set_clip_utf16(text: String) -> Result<()> {
//    set_clipboard(formats::Unicode, text)
//        .map_err(|e| anyhow::anyhow!("clipboard error: {:?}", e))
//}
pub fn set_clip_utf16(text: impl AsRef<str>) -> Result<()> {
    set_clipboard(formats::Unicode, text.as_ref().to_string())
        .map_err(|e| anyhow::anyhow!("clipboard error: {:?}", e))
}

// -----------------------------------------------------------------------------
// clear_clipboard
// -----------------------------------------------------------------------------
//
// クリップボードをクリアする。
// clipboard-win には「クリア専用 API」はないため、
// 空文字列を書き込むことで実質的なクリアとする。
// -----------------------------------------------------------------------------
pub fn clear_clipboard() -> Result<()> {
    set_clipboard(formats::Unicode, String::new())
        .map_err(|e| anyhow::anyhow!("clipboard clear error: {:?}", e))
}
