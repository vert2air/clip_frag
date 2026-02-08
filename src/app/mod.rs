// ============================================================================
// src/app/mod.rs
// ============================================================================
//
// このファイルは clip_frag アプリケーションの「中核モジュール」である。
// App 構造体を定義し、アプリケーション全体の状態遷移（main → finalize → exit）
// を制御する責務を持つ。
//
// ただし、実際のロジック（フラグメント構築、エンコード判定、TTY 入力、
// クリップボード操作など）は、すべてサブモジュールに委譲する。
// これにより App は「オーケストレーション」に集中でき、
// コードの見通しと保守性が大幅に向上する。
// ============================================================================

pub mod clipboard;
pub mod encoding;
pub mod fragment;
pub mod state;
pub mod tty;

use crate::Cli;
use anyhow::Result;

use clipboard::{clear_clipboard, set_clip_utf16};
use encoding::detect_encoding_and_decode;
use fragment::{build_fragment, calc_consumed_units, format_with_underscore};
use state::{AppState, Unit};
use tty::read_line_from_tty;

use std::fs::File;
use std::io::{self, Read};

// ============================================================================
// App 構造体
// ============================================================================
//
// App はアプリケーション全体の状態を保持し、
// main_loop → finalize_loop → exit_loop の流れを制御する。
// ============================================================================
pub struct App {
    /// アプリケーションの状態（prev_contents, curr_index など）
    pub state: AppState,
}

impl App {
    // ------------------------------------------------------------------------
    // App::new
    // ------------------------------------------------------------------------
    //
    // CLI 引数を受け取り、AppState を初期化する。
    // 入力データの読み込み、エンコード判定、行分割などもここで行う。
    //
    pub fn new(cli: Cli) -> Result<Self> {
        // ------------------------------------------------------------
        // 1. 最大データ量の決定
        // ------------------------------------------------------------
        let (unit, max_unit) = if let Some(c) = cli.chars {
            (Unit::Chars, c)
        } else if let Some(b) = cli.bytes {
            (Unit::Bytes, b)
        } else {
            (Unit::Chars, 10_240) // デフォルトは 10,240 文字
        };

        // ------------------------------------------------------------
        // 2. 入力データの読み込み
        // ------------------------------------------------------------
        let (input_text, from_file, input_file_name) =
            if let Some(path) = cli.input_file {
                // ファイルから読み込む
                let mut f = File::open(&path)?;
                let mut buf = Vec::new();
                f.read_to_end(&mut buf)?;

                // エンコード判定（UTF-8 / Shift_JIS）
                let (text, encoding_name) = detect_encoding_and_decode(&buf)?;
                eprintln!("encoding: {}", encoding_name);

                (text, true, Some(path.to_string_lossy().to_string()))
            } else {
                // 標準入力から読み込む
                let mut buf = Vec::new();
                io::stdin().read_to_end(&mut buf)?;

                let (text, encoding_name) = detect_encoding_and_decode(&buf)?;
                eprintln!("encoding: {}", encoding_name);

                (text, false, None)
            };

        // ------------------------------------------------------------
        // 3. AppState の初期化
        // ------------------------------------------------------------
        let mut state = AppState::new(
            input_text,
            unit,
            max_unit,
            from_file,
            input_file_name,
        );

        // ------------------------------------------------------------
        // 4. prev_contents の初期化（ファイル指定時のみ）
        // ------------------------------------------------------------
        if state.from_file {
            if let Some(ref name) = state.input_file_name {
                let header = format!(
                    "以下に、ファイル: {} を入力します。\n---\n",
                    name
                );
                set_clip_utf16(header.clone())?;
                state.prev_contents = header;
            }
        }

        Ok(Self { state })
    }

    // ------------------------------------------------------------------------
    // App::run
    // ------------------------------------------------------------------------
    //
    // アプリケーション本体の実行。
    // main_loop → finalize_loop（ファイル指定時のみ）→ exit_loop の順に進む。
    //
    pub fn run(&mut self) -> Result<()> {
        // main_loop（分割処理の本体）
        self.main_loop()?;

        // finalize_loop（ファイル指定時のみ）
        if self.state.from_file {
            self.finalize_loop()?;
        }

        // exit_loop（終了処理）
        self.exit_loop()?;

        Ok(())
    }

    // ------------------------------------------------------------------------
    // main_loop
    // ------------------------------------------------------------------------
    //
    // 分割処理の本体。
    // curr_index から始めて、最大データ量を超えない範囲で行を詰め込み、
    // ユーザに Yes/Prev/Quit を問い合わせる。
    //
    fn main_loop(&mut self) -> Result<()> {
        loop {
            // すでに全行を処理し終えている場合は終了
            if self.state.curr_index >= self.state.lines.len() {
                break;
            }

            // ------------------------------------------------------------
            // フラグメント構築
            // ------------------------------------------------------------
            let (fragment, fragment_units, next_index) =
                build_fragment(&self.state, self.state.curr_index);

            // ------------------------------------------------------------
            // 進捗計算
            // ------------------------------------------------------------
            let consumed_before =
                calc_consumed_units(&self.state, self.state.curr_index);
            let consumed_after = consumed_before + fragment_units;

            let percent_fragment = if self.state.total_units == 0 {
                0.0
            } else {
                (fragment_units as f64) * 100.0
                    / (self.state.total_units as f64)
            };

            let percent_cumulative = if self.state.total_units == 0 {
                0.0
            } else {
                (consumed_after as f64) * 100.0
                    / (self.state.total_units as f64)
            };

            let unit_label = match self.state.unit {
                Unit::Chars => "chars",
                Unit::Bytes => "bytes",
            };

            let frag_str = format_with_underscore(fragment_units);
            let total_str = format_with_underscore(self.state.total_units);
            let cumu_str = format_with_underscore(consumed_after);

            // ------------------------------------------------------------
            // プロンプト表示
            // ------------------------------------------------------------
            eprint!(
                "+{} [{}] ({:.1} %), {} / {} ({:.1} %): Y(es)/P(rev)/Q(uit) [y]: ",
                frag_str, unit_label, percent_fragment, cumu_str, total_str,
                percent_cumulative
            );

            // ------------------------------------------------------------
            // TTY からユーザ入力
            // ------------------------------------------------------------
            let input = read_line_from_tty()?.trim().to_string();
            let decision = if input.is_empty() { "y" } else { &input };

            match decision.to_lowercase().as_str() {
                "y" | "yes" => {
                    // Yes → fragment を clipboard に取り込む
                    set_clip_utf16(fragment.clone())?;
                    self.state.prev_contents = fragment;
                    self.state.curr_index = next_index;

                    if self.state.curr_index >= self.state.lines.len() {
                        break;
                    }
                }
                "p" | "prev" => {
                    // Prev → prev_contents を clipboard に取り込む
                    set_clip_utf16(self.state.prev_contents.clone())?;
                }
                "q" | "quit" => {
                    // Quit → clipboard をクリアして終了
                    clear_clipboard()?;
                    std::process::exit(0);
                }
                _ => {
                    eprintln!("無効な入力です。Y(es)/P(rev)/Q(uit) のいずれかを入力してください。");
                }
            }
        }

        Ok(())
    }

    // ------------------------------------------------------------------------
    // finalize_loop
    // ------------------------------------------------------------------------
    //
    // ファイル指定時のみ実行される「クローズ処理」。
    // 最後に「以上が、ファイル: <名前> の内容である。」を貼るかどうかを確認する。
    //
    fn finalize_loop(&mut self) -> Result<()> {
        loop {
            eprint!("+footer prompt: Y(es)/P(rev)/Q(uit) [y]: ");

            let input = read_line_from_tty()?.trim().to_string();
            let decision = if input.is_empty() { "y" } else { &input };

            match decision.to_lowercase().as_str() {
                "y" | "yes" => {
                    let footer = if let Some(ref name) =
                        self.state.input_file_name
                    {
                        format!("以上が、ファイル: {} の内容である。\n", name)
                    } else {
                        "以上が、入力データの内容である。\n".to_string()
                    };

                    set_clip_utf16(footer.clone())?;
                    self.state.prev_contents = footer;

                    break;
                }
                "p" | "prev" => {
                    set_clip_utf16(self.state.prev_contents.clone())?;
                }
                "q" | "quit" => {
                    clear_clipboard()?;
                    std::process::exit(0);
                }
                _ => {
                    eprintln!("無効な入力です。Y(es)/P(rev)/Q(uit) のいずれかを入力してください。");
                }
            }
        }

        Ok(())
    }

    // ------------------------------------------------------------------------
    // exit_loop
    // ------------------------------------------------------------------------
    //
    // 最終終了処理。
    // P(rev)/Q(uit) のみ。
    //
    fn exit_loop(&mut self) -> Result<()> {
        loop {
            eprint!("P(rev)/Q(uit) [q]: ");

            let input = read_line_from_tty()?.trim().to_string();
            let decision = if input.is_empty() { "q" } else { &input };

            match decision.to_lowercase().as_str() {
                "p" | "prev" => {
                    set_clip_utf16(self.state.prev_contents.clone())?;
                }
                "q" | "quit" => {
                    clear_clipboard()?;
                    std::process::exit(0);
                }
                _ => {
                    eprintln!("無効な入力です。P(rev)/Q(uit) のいずれかを入力してください。");
                }
            }
        }
    }
}
