// src/main.rs
//
// 分割clipアプリケーション本体。
// 指定されたテキストファイルを行単位で分割しながら、
// クリップボードに Unicode (UTF-16, CF_UNICODETEXT) で取り込む CLI ツール。
//
// 想定環境: Windows 11 + Rust + (PowerShell / MSYS2 など)
//
// 依存クレート（Cargo.toml に追記する想定）:
//
// [package]
// name = "clip_frag"
// version = "0.1.0"
// edition = "2021"
//
// [dependencies]
// clap = { version = "4", features = ["derive"] }
// encoding_rs = "0.8"
// clipboard-win = "5.4"
//
// ※ このファイルは UTF-8 で保存すること。

use std::fs;
use std::io::{self, BufRead, Write};
use std::path::PathBuf;

use clap::Parser;
use encoding_rs::SHIFT_JIS;

// clipboard-win v5.4 の API を使用する。
// - set_clipboard: 指定フォーマットでクリップボードにデータを書き込む関数
// - formats: Unicode / RawData などのフォーマット操作用
use clipboard_win::{formats, set_clipboard};
#[allow(unused_imports)]
use clipboard_win::formats::RawData; // 仕様上の指定に従い import（本コードでは書き込みには使用しない）

/// データ量の単位を表す列挙型。
/// - Chars: 文字数ベース
/// - Bytes: バイト数ベース（ここでは UTF-16 のバイト数を想定）
#[derive(Debug, Clone, Copy)]
enum UnitKind {
    Chars,
    Bytes,
}

/// CLI 引数定義。
/// clip_frag [-c <文字数>|-b <byte数>] <入力ファイル名>
#[derive(Parser, Debug)]
#[command(name = "clip_frag")]
#[command(about = "行指向テキストファイルを分割しながらクリップボードに取り込むツール", long_about = None)]
struct Cli {
    /// 一回に取り込む最大文字数
    #[arg(short = 'c', value_name = "文字数", conflicts_with = "bytes")]
    chars: Option<usize>,

    /// 一回に取り込む最大バイト数（UTF-16 のバイト数を想定）
    #[arg(short = 'b', value_name = "byte数", conflicts_with = "chars")]
    bytes: Option<usize>,

    /// 入力ファイル名
    #[arg(value_name = "入力ファイル名")]
    input_file: PathBuf,
}

/// アプリケーション全体で共有する状態。
struct AppState {
    /// 入力ファイル名（表示用）
    input_file_name: String,
    /// 入力ファイルの全行（行末の改行も含めて保持する）
    lines: Vec<String>,
    /// 各行の「単位あたりの長さ」（文字数またはバイト数）
    line_units: Vec<usize>,
    /// 入力ファイル全体のデータ量（単位は UnitKind に依存）
    total_units: usize,
    /// これまでに処理した累積データ量
    cumulative_units: usize,
    /// 一回に取り込む最大データ量
    max_units_per_fragment: usize,
    /// データ量の単位種別
    unit_kind: UnitKind,
    /// 直前にクリップボードに取り込んだ内容
    prev_contents: String,
    /// 次に取り込むべき行のインデックス（0 始まり）
    curr_index: usize,
}

impl AppState {
    /// 単位種別に応じた単位名を返す。
    fn unit_label(&self) -> &'static str {
        match self.unit_kind {
            UnitKind::Chars => "chars",
            UnitKind::Bytes => "bytes",
        }
    }
}

/// 数値を 3 桁ごとにアンダースコアで区切った文字列に変換する。
/// 例: 10240 -> "10_240"
fn format_with_underscores(n: usize) -> String {
    let s = n.to_string();
    let mut result = String::new();
    let chars: Vec<char> = s.chars().collect();
    let len = chars.len();
    for (i, ch) in chars.into_iter().enumerate() {
        result.push(ch);
        let pos_from_end = len - i - 1;
        if pos_from_end > 0 && pos_from_end % 3 == 0 {
            result.push('_');
        }
    }
    result
}

/// 標準入力から 1 行読み取り、末尾の改行を取り除いた文字列を返す。
fn read_line_from_stdin() -> io::Result<String> {
    let stdin = io::stdin();
    let mut line = String::new();
    stdin.lock().read_line(&mut line)?;
    if line.ends_with('\n') {
        line.pop();
        if line.ends_with('\r') {
            line.pop();
        }
    }
    Ok(line)
}

/// clipboard-win のエラー型を std::io::Error に変換するためのヘルパ。
fn map_clipboard_err<E: std::fmt::Debug>(e: E, context: &str) -> io::Error {
    io::Error::new(io::ErrorKind::Other, format!("{}: {:?}", context, e))
}

/// クリップボードをクリアする。
///
/// 実装方針:
/// - set_clipboard(formats::Unicode, "") を使って空文字列を書き込むことで、
///   「空のテキスト」として扱わせる。
fn clear_clipboard() -> io::Result<()> {
    set_clipboard(formats::Unicode, "")
        .map_err(|e| map_clipboard_err(e, "Clipboard clear error"))
}

/// テキストを Unicode (UTF-16, CF_UNICODETEXT) として Windows クリップボードに設定する。
///
/// clipboard-win v5.4 では、
///   set_clipboard(formats::Unicode, text)?;
/// というスタイルで使用する。
fn set_clipboard_unicode(text: &str) -> io::Result<()> {
    set_clipboard(formats::Unicode, text)
        .map_err(|e| map_clipboard_err(e, "Clipboard set error"))
}

/// 入力ファイルのエンコードを判定し、UTF-8 の String として返す。
/// 想定エンコードは UTF-8 または SHIFT-JIS。
fn read_and_detect_encoding(path: &PathBuf) -> io::Result<(String, &'static str)> {
    let bytes = fs::read(path)?;

    // まず UTF-8 として解釈を試みる
    match String::from_utf8(bytes.clone()) {
        Ok(s) => {
            // UTF-8 と判定
            eprintln!("入力ファイルのエンコードを判定しました: UTF-8");
            Ok((s, "UTF-8"))
        }
        Err(_) => {
            // UTF-8 として解釈できなかった場合、SHIFT-JIS とみなしてデコード
            let (cow, _, had_errors) = SHIFT_JIS.decode(&bytes);
            if had_errors {
                // SHIFT-JIS としてもおかしい場合はエラーとする
                Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "入力ファイルのエンコードを UTF-8 でも SHIFT-JIS でも解釈できませんでした",
                ))
            } else {
                let s: String = cow.into_owned();
                eprintln!("入力ファイルのエンコードを判定しました: SHIFT-JIS");
                Ok((s, "SHIFT-JIS"))
            }
        }
    }
}

/// テキスト全体を「行単位」に分割する。
/// 行末の改行文字も含めて保持するため、split_inclusive を利用する。
fn split_lines_with_terminator(text: &str) -> Vec<String> {
    // 最後の行に改行がない場合も、そのまま 1 行として扱う。
    let mut lines: Vec<String> = text
        .split_inclusive('\n')
        .map(|s| s.to_string())
        .collect();

    // もし行が空で、かつテキストが空でない場合は、そのまま 1 行として扱う。
    if lines.is_empty() && !text.is_empty() {
        lines.push(text.to_string());
    }

    lines
}

/// 各行の「単位あたりの長さ」を計算する。
/// - UnitKind::Chars の場合: line.chars().count()
/// - UnitKind::Bytes の場合: UTF-16 のバイト数（u16 * 2）
///
/// ※ UTF-16 のバイト数は、Windows の CF_UNICODETEXT の実際のサイズに近い指標となる。
fn compute_line_units(lines: &[String], unit_kind: UnitKind) -> Vec<usize> {
    lines
        .iter()
        .map(|line| match unit_kind {
            UnitKind::Chars => line.chars().count(),
            UnitKind::Bytes => {
                // UTF-16 のコードユニット数 * 2 バイト
                let utf16_len = line.encode_utf16().count();
                utf16_len * 2
            }
        })
        .collect()
}

/// 指定された curr_index から始めて、
/// 一回に取り込む最大データ量を超えない範囲で、
/// 何行取り込むかを決定し、その行数とデータ量を返す。
fn decide_fragment(state: &AppState) -> (usize, usize) {
    let mut units_sum = 0usize;
    let mut count_lines = 0usize;

    for idx in state.curr_index..state.lines.len() {
        let line_units = state.line_units[idx];

        if count_lines == 0 {
            // 最初の行は、たとえ max_units_per_fragment を超えていても、
            // そのまま 1 行として取り込むことにする。
            // （仕様上は「最大データ量を超えない」ことが理想だが、
            //  行を分割しないという制約を優先するための妥協策）
            if line_units > state.max_units_per_fragment {
                units_sum = line_units;
                count_lines = 1;
                break;
            }
        }

        if units_sum + line_units > state.max_units_per_fragment {
            // 次の行を追加すると最大データ量を超えるので、ここで打ち切る
            break;
        }

        units_sum += line_units;
        count_lines += 1;
    }

    (count_lines, units_sum)
}

/// フラグメントのテキストを構築する。
fn build_fragment_text(lines: &[String], start: usize, count: usize) -> String {
    let end = start + count;
    lines[start..end].join("")
}

/// メインのループ処理（本体の分割処理）を実行する。
fn main_loop(state: &mut AppState) -> io::Result<()> {
    // 標準エラー出力を明示的に扱うためのハンドル
    let stderr = io::stderr();
    let mut stderr_lock = stderr.lock();

    while state.curr_index < state.lines.len() {
        // 次に取り込むフラグメントを決定
        let (line_count, fragment_units) = decide_fragment(state);

        if line_count == 0 {
            // 取り込む行がない場合は、ループを抜けてファイナライズへ
            break;
        }

        // フラグメントのテキストを構築
        let fragment_text =
            build_fragment_text(&state.lines, state.curr_index, line_count);

        // このフラグメントを処理した後の累積データ量
        let new_cumulative = state.cumulative_units + fragment_units;

        // 入力ファイル全体に対する割合（フラグメント単体）
        let percent_fragment = if state.total_units > 0 {
            (fragment_units as f64) * 100.0 / (state.total_units as f64)
        } else {
            0.0
        };

        // 入力ファイル全体に対する割合（累積）
        let percent_cumulative = if state.total_units > 0 {
            (new_cumulative as f64) * 100.0 / (state.total_units as f64)
        } else {
            0.0
        };

        // プロンプト文字列を構築
        let fragment_units_str = format_with_underscores(fragment_units);
        let cumulative_str = format_with_underscores(new_cumulative);
        let total_str = format_with_underscores(state.total_units);
        let unit_label = state.unit_label();

        // 例:
        // +10_240 [chars] (25.0 %), 10_240 / 40_960 (25.0 %): Y(es)/P(rev)/Q(uit) [y]:
        writeln!(
            stderr_lock,
            "+{} [{}] ({:.1} %), {} / {} ({:.1} %): Y(es)/P(rev)/Q(uit) [y]:",
            fragment_units_str,
            unit_label,
            percent_fragment,
            cumulative_str,
            total_str,
            percent_cumulative
        )?;
        stderr_lock.flush()?;

        // 標準入力からユーザの選択を受け取る
        let input = read_line_from_stdin()?;
        let trimmed = input.trim();
        let lower = trimmed.to_ascii_lowercase();

        if lower.is_empty() || lower == "y" || lower == "yes" {
            // Yes: 新たに取り込むデータをクリップボードに取り込む
            set_clipboard_unicode(&fragment_text)?;
            // prev_contents を更新
            state.prev_contents = fragment_text;
            // curr_index を進める
            state.curr_index += line_count;
            // 累積データ量を更新
            state.cumulative_units = new_cumulative;

            // ファイル末尾に達した場合はファイナライズ処理へ
            if state.curr_index >= state.lines.len() {
                break;
            }
        } else if lower == "p" || lower == "prev" {
            // Prev: 直前のデータを再度クリップボードに取り込む
            set_clipboard_unicode(&state.prev_contents)?;
            // curr_index や cumulative_units は変更しない
            continue;
        } else if lower == "q" || lower == "quit" {
            // Quit: クリップボードをクリアして終了
            clear_clipboard()?;
            return Ok(());
        } else {
            // 不正な入力の場合は、何もせず再度ループ
            writeln!(
                stderr_lock,
                "無効な入力です。Y(es)/P(rev)/Q(uit) のいずれかを指定してください。"
            )?;
            stderr_lock.flush()?;
            continue;
        }
    }

    Ok(())
}

/// ファイナライズ処理（フッタープロンプト）
///
/// プロンプト:
/// +footer prompt: Y(es)/P(rev)/Q(uit) [y]:
fn finalize_loop(state: &mut AppState) -> io::Result<()> {
    let stderr = io::stderr();
    let mut stderr_lock = stderr.lock();

    loop {
        writeln!(
            stderr_lock,
            "+footer prompt: Y(es)/P(rev)/Q(uit) [y]:"
        )?;
        stderr_lock.flush()?;

        let input = read_line_from_stdin()?;
        let trimmed = input.trim();
        let lower = trimmed.to_ascii_lowercase();

        if lower.is_empty() || lower == "y" || lower == "yes" {
            // yes が選択された場合、フッターテキストをクリップボードに取り込む
            let footer = format!(
                "以上が、ファイル: {} の内容である。\n",
                state.input_file_name
            );
            set_clipboard_unicode(&footer)?;
            state.prev_contents = footer;
            // 終了処理へ進む
            break;
        } else if lower == "p" || lower == "prev" {
            // prev が選択された場合、prev_contents をクリップボードに取り込む
            set_clipboard_unicode(&state.prev_contents)?;
            // 再度ファイナライズ処理を実行（ループ継続）
            continue;
        } else if lower == "q" || lower == "quit" {
            // quit の場合、クリップボードをクリアして終了
            clear_clipboard()?;
            return Ok(());
        } else {
            writeln!(
                stderr_lock,
                "無効な入力です。Y(es)/P(rev)/Q(uit) のいずれかを指定してください。"
            )?;
            stderr_lock.flush()?;
            continue;
        }
    }

    Ok(())
}

/// 終了処理ループ。
///
/// プロンプト:
/// P(rev)/Q(uit) [q]:
fn exit_loop(state: &mut AppState) -> io::Result<()> {
    let stderr = io::stderr();
    let mut stderr_lock = stderr.lock();

    loop {
        writeln!(stderr_lock, "P(rev)/Q(uit) [q]:")?;
        stderr_lock.flush()?;

        let input = read_line_from_stdin()?;
        let trimmed = input.trim();
        let lower = trimmed.to_ascii_lowercase();

        if lower == "p" || lower == "prev" {
            // prev が選択された場合、prev_contents をクリップボードに取り込む
            set_clipboard_unicode(&state.prev_contents)?;
            // 再度終了処理を実行（ループ継続）
            continue;
        } else if lower.is_empty() || lower == "q" || lower == "quit" {
            // quit（またはデフォルト）の場合、クリップボードをクリアして終了
            clear_clipboard()?;
            return Ok(());
        } else {
            writeln!(
                stderr_lock,
                "無効な入力です。P(rev)/Q(uit) のいずれかを指定してください。"
            )?;
            stderr_lock.flush()?;
            continue;
        }
    }
}

fn main() -> io::Result<()> {
    // CLI 引数をパース
    let cli = Cli::parse();

    // 一回に取り込む最大データ量と単位種別を決定
    let (max_units_per_fragment, unit_kind) = if let Some(c) = cli.chars {
        (c, UnitKind::Chars)
    } else if let Some(b) = cli.bytes {
        (b, UnitKind::Bytes)
    } else {
        // デフォルトは 10,240 文字
        (10_240, UnitKind::Chars)
    };

    // 入力ファイルを読み込み、エンコードを判定して UTF-8 の String として取得
    let (file_text, _encoding_name) = read_and_detect_encoding(&cli.input_file)?;

    // 行単位に分割（行末の改行も含める）
    let lines = split_lines_with_terminator(&file_text);

    // 各行の単位あたりの長さを計算
    let line_units = compute_line_units(&lines, unit_kind);

    // 入力ファイル全体のデータ量を計算
    let total_units: usize = line_units.iter().copied().sum();

    // 入力ファイル名（表示用）を文字列として取得
    let input_file_name = cli
        .input_file
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("<unknown>")
        .to_string();

    // アプリ起動時のヘッダプロンプトを構築
    let header = format!(
        "以下に、ファイル: {} を入力します。\n---\n",
        input_file_name
    );

    // ヘッダプロンプトをクリップボードに取り込む（Unicode）
    set_clipboard_unicode(&header)?;

    // prev_contents にヘッダを記憶
    let prev_contents = header;

    // curr_index を 0 に設定
    let curr_index = 0usize;

    // アプリケーション状態を構築
    let mut state = AppState {
        input_file_name,
        lines,
        line_units,
        total_units,
        cumulative_units: 0,
        max_units_per_fragment,
        unit_kind,
        prev_contents,
        curr_index,
    };

    // 本体のループ処理を実行
    main_loop(&mut state)?;

    // ファイナライズ処理
    finalize_loop(&mut state)?;

    // 終了処理
    exit_loop(&mut state)?;

    Ok(())
}
