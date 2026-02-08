//! ============================================================================
//! src/app/mod.rs
//! ============================================================================
//!
//! clip_frag のアプリケーションロジックの中心。
//!
//! App 構造体は「状態遷移のオーケストレーション」を担当し、
//! 実際の処理（分割・エンコード判定・TTY 入力・クリップボード操作）は
//! すべてサブモジュールに委譲する。
//!
//! 設計思想：
//!   - App は「状態遷移の制御」に専念する。
//!   - 純粋ロジックは fragment.rs / encoding.rs / state.rs に分離。
//!   - I/O は tty.rs / clipboard.rs に分離。
//!   - CLI パーサ（Cli）は main.rs に閉じ込める。
//!
//! これにより、テスト容易性・保守性・責務分離が最大化される。
//!
//! ============================================================================

pub mod clipboard;
pub mod encoding;
pub mod fragment;
pub mod state;
pub mod tty;

pub use state::Unit;

use anyhow::Result;

use clipboard::{clear_clipboard, set_clip_utf16};
use fragment::{build_fragment, calc_consumed_units, format_with_underscore};
use state::AppState;
use tty::read_line_from_tty;

// ============================================================================
// App 構造体
// ============================================================================
//
// App はアプリケーション全体の状態（AppState）を保持し、
// main_loop → finalize_loop → exit_loop の流れを制御する。
// ============================================================================
pub struct App {
    /// アプリケーションの状態（行データ・進捗・前回内容など）
    pub state: AppState,
}

impl App {
    // ------------------------------------------------------------------------
    // App::new
    // ------------------------------------------------------------------------
    //
    // CLI に依存しない純粋な初期化関数。
    // main.rs 側で読み込んだ入力データと設定値を受け取り、
    // AppState を構築する。
    //
    // ファイル指定時はヘッダを clipboard に入れる。
    // ------------------------------------------------------------------------
    pub fn new(
        input_text: String,
        unit: Unit,
        max_unit: usize,
        from_file: bool,
        input_file_name: Option<String>,
    ) -> Result<Self> {
        // AppState の構築（行分割・単位計算など）
        let mut state = AppState::new(
            input_text,
            unit,
            max_unit,
            from_file,
            input_file_name,
        );

        // ファイル指定時はヘッダを clipboard に入れる
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
    // アプリケーションのメインフロー。
    // main_loop → finalize_loop（ファイル指定時）→ exit_loop の順に進む。
    // ------------------------------------------------------------------------
    pub fn run(&mut self) -> Result<()> {
        self.main_loop()?;

        if self.state.from_file {
            self.finalize_loop()?;
        }

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
    // fragment.rs の build_fragment が純粋ロジックとして分割を担当する。
    // ------------------------------------------------------------------------
    fn main_loop(&mut self) -> Result<()> {
        loop {
            // 全行処理済みなら終了
            if self.state.curr_index >= self.state.lines.len() {
                break;
            }

            // フラグメント構築（純粋ロジック）
            let (fragment, fragment_units, next_index) =
                build_fragment(&self.state, self.state.curr_index);

            // 進捗計算（純粋ロジック）
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

            // プロンプト表示
            eprint!(
                "+{} [{}] ({:.1} %), {} / {} ({:.1} %): Y(es)/P(rev)/Q(uit) [y]: ",
                frag_str, unit_label, percent_fragment,
                cumu_str, total_str, percent_cumulative
            );

            // TTY 入力（tty.rs）
            let input = read_line_from_tty()?.trim().to_string();
            let decision = if input.is_empty() { "y" } else { &input };

            match decision.to_lowercase().as_str() {
                "y" | "yes" => {
                    // fragment を clipboard に取り込む
                    set_clip_utf16(fragment.clone())?;
                    self.state.prev_contents = fragment;
                    self.state.curr_index = next_index;

                    if self.state.curr_index >= self.state.lines.len() {
                        break;
                    }
                }
                "p" | "prev" => {
                    // 前回内容を clipboard に戻す
                    set_clip_utf16(self.state.prev_contents.clone())?;
                }
                "q" | "quit" => {
                    // 終了
                    clear_clipboard()?;
                    std::process::exit(0);
                }
                _ => {
                    eprintln!("無効な入力です。Y(es)/P(rev)/Q(uit) を入力してください。");
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
    // 最後に「以上が、ファイル: <名前> の内容である。」を貼るかどうか確認する。
    // ------------------------------------------------------------------------
    fn finalize_loop(&mut self) -> Result<()> {
        loop {
            eprint!("+footer prompt: Y(es)/P(rev)/Q(uit) [y]: ");

            let input = read_line_from_tty()?.trim().to_string();
            let decision = if input.is_empty() { "y" } else { &input };

            match decision.to_lowercase().as_str() {
                "y" | "yes" => {
                    let footer = if let Some(ref name) = self.state.input_file_name {
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
                    eprintln!("無効な入力です。Y(es)/P(rev)/Q(uit) を入力してください。");
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
    // ------------------------------------------------------------------------
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
                    eprintln!("無効な入力です。P(rev)/Q(uit) を入力してください。");
                }
            }
        }
    }
}
