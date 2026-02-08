// ============================================================================
// src/app/fragment.rs
// ============================================================================
//
// このファイルでは、clip_frag の「分割処理」の中心となるロジックを提供する。
// 具体的には以下の 3 つの責務を持つ：
//
//   1. build_fragment()
//      - curr_index から始めて、最大データ量を超えない範囲で行を詰め込む。
//      - 行は「丸ごと含める or 全く含めない」のどちらかで、途中分割はしない。
//      - fragment（取り込むテキスト）、fragment_units（単位数）、next_index を返す。
//
//   2. calc_consumed_units()
//      - curr_index までに消費した単位数（chars or bytes）を計算する。
//
//   3. format_with_underscore()
//      - 10_240 のようにアンダースコア付きで数値をフォーマットする。
//        プロンプト表示を読みやすくするための補助関数。
//
// AppState のデータ構造は state.rs に定義されており、
// ここではそれを参照して純粋なロジックだけを提供する。
// ============================================================================

use super::state::AppState;

// -----------------------------------------------------------------------------
// build_fragment
// -----------------------------------------------------------------------------
//
// curr_index から始めて、最大データ量（chars or bytes）を超えない範囲で
// 行を詰め込んだフラグメントを構築する。
//
// 仕様：
//   - 行は途中で分割しない。
//   - 1 行追加すると max_unit を超える場合、その行は含めない。
//   - fragment（String）、fragment_units（usize）、next_index（usize）を返す。
// -----------------------------------------------------------------------------
pub fn build_fragment(
    state: &AppState,
    start_index: usize,
) -> (String, usize, usize) {
    let mut fragment = String::new();
    let mut used_units = 0usize;
    let mut idx = start_index;

    while idx < state.lines.len() {
        let line = &state.lines[idx];
        let line_units = state.line_units[idx];

        // 次の行を追加すると最大データ量を超える場合は、その行は含めない。
        if used_units + line_units > state.max_unit {
            break;
        }

        fragment.push_str(line);
        used_units += line_units;
        idx += 1;
    }

    (fragment, used_units, idx)
}

// -----------------------------------------------------------------------------
// calc_consumed_units
// -----------------------------------------------------------------------------
//
// curr_index までに消費した単位数（chars or bytes）を計算する。
// これは進捗表示（累積 %）のために必要。
// -----------------------------------------------------------------------------
pub fn calc_consumed_units(state: &AppState, curr_index: usize) -> usize {
    state.line_units.iter().take(curr_index).sum()
}

// -----------------------------------------------------------------------------
// format_with_underscore
// -----------------------------------------------------------------------------
//
// 数値をアンダースコア付きでフォーマットする。
// 例：10240 → "10_240"
//
// Rust の標準フォーマットにはアンダースコア区切りがないため、
// 手動で 3 桁ごとに '_' を挿入する。
// -----------------------------------------------------------------------------
pub fn format_with_underscore(n: usize) -> String {
    let s = n.to_string();
    let bytes = s.as_bytes();
    let len = bytes.len();
    let mut out = String::new();

    for (i, ch) in s.chars().enumerate() {
        out.push(ch);
        let pos_from_end = len - i - 1;
        if pos_from_end > 0 && pos_from_end.is_multiple_of(3) {
            out.push('_');
        }
    }

    out
}
