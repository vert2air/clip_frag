//! ============================================================================
//! src/main.rs
//! ============================================================================
//!
//! このファイルは clip_frag の「エントリーポイント」であり、
//! CLI 引数のパースと、App（アプリケーション本体）の起動のみを担当する。
//!
//! 重要な設計方針：
//!   - main.rs は「CLI と OS 入出力」のみを扱う。
//!   - アプリケーションロジック（分割処理・状態遷移）は app モジュールに集約する。
//!   - これにより、テストは app 側に集中でき、main.rs は最小限の責務で済む。
//!
//! ============================================================================

use anyhow::Result;
use clap::Parser;

use clip_frag::app::{App, Unit};

/// CLI オプション定義
///
/// clap による自動パーサ。App 側には依存させず、
/// main.rs 内に閉じ込めることで責務を明確化している。
#[derive(Parser, Debug)]
#[command(author, version, about)]
pub struct Cli {
    /// 一回に取り込む最大文字数
    #[arg(short = 'c', long = "chars")]
    pub chars: Option<usize>,

    /// 一回に取り込む最大バイト数
    #[arg(short = 'b', long = "bytes")]
    pub bytes: Option<usize>,

    /// 入力ファイル名（省略時は標準入力）
    pub input_file: Option<std::path::PathBuf>,
}

fn main() -> Result<()> {
    // -------------------------------------------------------------------------
    // 1. CLI 引数のパース
    // -------------------------------------------------------------------------
    let cli = Cli::parse();

    // -------------------------------------------------------------------------
    // 2. 最大データ量の決定
    //    chars / bytes のどちらかが指定されていればそれを採用。
    //    どちらも指定されていなければデフォルト 10,240 chars。
    // -------------------------------------------------------------------------
    let (unit, max_unit) = if let Some(c) = cli.chars {
        (Unit::Chars, c)
    } else if let Some(b) = cli.bytes {
        (Unit::Bytes, b)
    } else {
        (Unit::Chars, 10_240)
    };

    // -------------------------------------------------------------------------
    // 3. 入力データの読み込み
    //    - ファイル指定時：ファイルから読み込み
    //    - 標準入力：stdin から読み込み
    //
    //    読み込んだバイト列は encoding.rs の detect_encoding_and_decode に渡し、
    //    UTF-8 または Shift_JIS としてデコードする。
    // -------------------------------------------------------------------------
    let (input_text, from_file, input_file_name) = if let Some(path) = cli.input_file {
        // ファイル入力
        let mut f = std::fs::File::open(&path)?;
        let mut buf = Vec::new();
        use std::io::Read;
        f.read_to_end(&mut buf)?;

        let (text, encoding_name) =
            clip_frag::app::encoding::detect_encoding_and_decode(&buf)?;
        eprintln!("encoding: {}", encoding_name);

        (text, true, Some(path.to_string_lossy().to_string()))
    } else {
        // 標準入力
        let mut buf = Vec::new();
        use std::io::Read;
        std::io::stdin().read_to_end(&mut buf)?;

        let (text, encoding_name) =
            clip_frag::app::encoding::detect_encoding_and_decode(&buf)?;
        eprintln!("encoding: {}", encoding_name);

        (text, false, None)
    };

    // -------------------------------------------------------------------------
    // 4. App の初期化
    //    App::new は CLI に依存しない純粋ロジック。
    // -------------------------------------------------------------------------
    let mut app = App::new(
        input_text,
        unit,
        max_unit,
        from_file,
        input_file_name,
    )?;

    // -------------------------------------------------------------------------
    // 5. 実行
    // -------------------------------------------------------------------------
    app.run()
}
