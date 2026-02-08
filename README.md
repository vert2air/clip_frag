# 分割clip — `clip_frag`

LLM や入力制限のあるフォームに長文を貼り付けたいとき、  
**行単位で安全に分割しながら clipboard に送れる CLI ツール**です。

- 行の途中で分割されない  
- 1 回に取り込む最大データ量を文字数または byte 数で指定可能  
- 「次へ」「前へ」「終了」の 3 操作だけで快適に分割ペースト  
- UTF-8 / Shift-JIS の自動判定  
- LLM へのコード貼り付けに最適

---

## ✨ Features

### 行単位の安全な分割
指定した最大データ量（文字数 or byte 数）を超えない範囲で、  
**行を丸ごと含める／含めない**のどちらかで分割します。

デフォルトの最大データ量は **10,240 文字**。

### クリップボード操作のループ
各フラグメントごとに以下のようなプロンプトが表示されます：

```
+10_240 [chars] (25.0 %), 10_240 / 40_960 (25.0 %): Y(es)/P(rev)/Q(uit) [y]:
```

操作は 3 つだけ：

- **Y / Enter**: 次のフラグメントを clipboard に取り込む  
- **P**: 直前のフラグメントを再度 clipboard に取り込む  
- **Q**: clipboard をクリアして終了  

### エンコード自動判定
- UTF-8 / Shift-JIS を自動判定  
- 判定結果は標準エラー出力に表示  

### 入力方法
- **ファイルパスを指定**  
- **標準入力から受け取る**

ファイル指定時は、冒頭に以下のヘッダが clipboard に入ります：

```
以下に、ファイル: <入力ファイル名> を入力します。
---
```

標準入力の場合はヘッダなし。

### ファイナライズ処理（ファイル入力時のみ）
最後のフラグメント後に以下のプロンプト：

```
+footer prompt: Y(es)/P(rev)/Q(uit) [y]:
```

Yes の場合、以下を clipboard に取り込みます：

```
以上が、ファイル: <入力ファイル名> の内容である。
```

---

## 📦 Install

### 1. Cargo からインストール（推奨）

Rust がインストールされている場合、Cargo で導入できます。

```bash
cargo install clip_frag
```

インストール後：

```bash
clip_frag --help
```

### 2. ソースコードからビルド

```bash
git clone https://github.com/<your-account>/clip_frag.git
cd clip_frag
cargo build --release
```

生成されたバイナリ：

```
target/release/clip_frag
```

---

## 🚀 Usage

### 基本構文

```bash
clip_frag [-c <文字数> | -b <byte数>] [<入力ファイル名>]
```

### 例

#### ファイルを 10,240 文字ごとに分割して貼り付け
```bash
clip_frag my_source.rs
```

#### 5,000 文字ごとに分割
```bash
clip_frag -c 5000 my_source.rs
```

#### 標準入力を使う
```bash
cat long.txt | clip_frag -b 8000
```

---

## 🧭 想定ユースケース

- LLM に長いソースコードを貼り付けたい  
- 入力制限のあるフォームに長文を分割して貼り付けたい  
- 行途中で切れると困るデータを扱いたい  
- Shift-JIS の古いファイルを扱う必要がある  

---

## 📄 License

This project is licensed under the **MIT License**.
詳細は LICENSE ファイルを参照してください。

