//! ============================================================================
//! src/app/clipboard.rs
//! ============================================================================
//!
//! このモジュールは、Windows / macOS / Linux のすべてで動作する
//! クロスプラットフォームなクリップボード操作を提供する。
//!
//! 以前は clipboard-win を使用していたが、Windows 専用であり
//! CI（Ubuntu）や macOS では動作しなかった。
//!
//! 現在は arboard を採用し、以下のメリットを得ている：
//!   - Windows / macOS / Linux すべてで動作
//!   - UTF-8 ベースで Rust の String と相性が良い
//!   - エラー型が std::error::Error を実装しており anyhow と相性抜群
//!   - OS ごとの API を意識せずに統一的に扱える
//!
//! clip_frag の設計思想（責務分離・安全ラップ）に完全に一致する。
//!
//! ============================================================================

use anyhow::{Context, Result};
use arboard::Clipboard;

// -----------------------------------------------------------------------------
// set_clip_utf16（UTF-8 ベースのクロスプラットフォーム版）
// -----------------------------------------------------------------------------
//
// Windows では UTF-16 が内部的に使われるが、arboard は UTF-8 を受け付けるため
// UTF-16 を意識する必要はない。
//
// - クリップボードに text を設定する
// - OS に依存しない
// - anyhow::Result で安全に扱える
// -----------------------------------------------------------------------------
pub fn set_clip_utf16(text: impl AsRef<str>) -> Result<()> {
    let mut clipboard =
        Clipboard::new().context("failed to open clipboard")?;

    clipboard
        .set_text(text.as_ref().to_string())
        .context("failed to set clipboard text")?;

    Ok(())
}

// -----------------------------------------------------------------------------
// clear_clipboard
// -----------------------------------------------------------------------------
//
// クリップボードを空文字列で上書きすることでクリアする。
// Windows / macOS / Linux すべてで動作する。
// -----------------------------------------------------------------------------
pub fn clear_clipboard() -> Result<()> {
    let mut clipboard =
        Clipboard::new().context("failed to open clipboard")?;

    clipboard.set_text(String::new()).context("failed to clear clipboard")?;

    Ok(())
}
