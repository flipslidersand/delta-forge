# Spec — DeltaForge

## プロジェクトの目的

入力の一部が変更された際に、処理全体ではなく影響を受ける部分だけを再計算するエンジン。
スプレッドシート・ビルドシステム・データパイプラインのコアにある「差分更新」の仕組みを自前実装する。

## 解決する問題

| 問題                                   | DeltaForge での解決策                                |
| -------------------------------------- | ---------------------------------------------------- |
| 入力変更のたびに全体を再計算するコスト | 依存グラフで影響範囲を特定し、必要なノードだけ再計算 |
| 再計算順序の誤り                       | トポロジカルソートで正しい実行順を保証               |
| どこを再計算したか追跡できない         | 実行ログに再計算済みノードと結果を記録               |

## 利用イメージ

```rust
let mut engine = DeltaForge::new();

let sales    = engine.input("sales", vec![...]);
let filtered = engine.map("filtered", &[sales], |inputs| {
    inputs[0].iter().filter(|r| r.active).collect()
});
let total    = engine.reduce("total", &[filtered], |inputs| {
    inputs[0].iter().map(|r| r.amount).sum::<i64>()
});

engine.update("sales", new_rows);
engine.recompute();   // filtered と total だけ再計算される
```

## MVP の境界線

### やること (Phase 1〜4)

- ノード登録・依存関係の定義
- 値の更新 (`update`) で upstream から dirty フラグを伝播
- トポロジカルソートによる再計算順序の決定
- dirty なノードだけ再計算 (ユーザー定義クロージャを実行)
- 実行ログ (どのノードが再計算されたか)

### やらないこと (Phase 1)

- 永続キャッシュ (SQLite)
- Rayon 並列再計算
- 分散ワーカー
- Web UI
- 循環依存の自動解消

## 成功条件

| Phase   | 完成条件                                                       |
| ------- | -------------------------------------------------------------- |
| Phase 1 | ノード登録・依存追加・DAG 構築のテスト全通過                   |
| Phase 2 | `update()` で dirty フラグが downstream に伝播するテスト全通過 |
| Phase 3 | `recompute()` がトポロジカル順に dirty ノードだけ再計算する    |
| Phase 4 | 実行ログに再計算ノード・所要時間が記録される                   |
| Phase 5 | SQLite に計算結果をキャッシュし、プロセス再起動後も復元できる  |
| Phase 6 | Rayon 並列再計算で Phase 3 より高速化 (独立ノード群で測定)     |
