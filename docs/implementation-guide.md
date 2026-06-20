# Implementation Guide — DeltaForge

## Phase 1: コアグラフ構築 (1週)

### 実装内容

- `src/engine.rs` — `DeltaForge` 構造体・`input()`・`compute()`・`add_dep()`
- `src/node.rs` — `Node`, `NodeKind` の定義
- petgraph の `DiGraph` でノードと依存エッジを管理

### 完成条件

```bash
cargo test engine::graph   # ノード登録・依存追加・循環検出テスト全通過
```

### 難所

- `Box<dyn Any + Send>` による型消去: 値を Any に入れ、出力時に `downcast_ref` で取り出す
- 循環依存の検出: petgraph の `is_cyclic_directed()` を `add_dep` 後に呼ぶ

---

## Phase 2: Dirty フラグ伝播 (1週)

### 実装内容

- `src/scheduler.rs` — `propagate_dirty(start: NodeId)`
- BFS (petgraph の `Bfs`) で start から downstream を全て dirty にする

### 完成条件

```bash
cargo test scheduler::propagate   # dirty 伝播テスト全通過
# input → A → B → C の場合、input 更新で A/B/C が全て dirty
```

---

## Phase 3: トポロジカル再計算 (1週)

### 実装内容

- `src/scheduler.rs` — `topo_recompute()`
- petgraph の `toposort()` で実行順を決定
- dirty なノードだけクロージャを呼んでキャッシュを更新

### 完成条件

```rust
let mut engine = DeltaForge::new();
let a = engine.input("a", 10i64);
let b = engine.compute("b", &[a], |inputs| {
    let v = inputs[0].downcast_ref::<i64>().unwrap();
    Box::new(v * 2)
});
engine.update("a", 20i64);
engine.recompute();
assert_eq!(*engine.get::<i64>("b").unwrap(), 40i64);
```

---

## Phase 4: 実行ログ (3日)

### 実装内容

- `src/log.rs` — `LogEntry`, `ExecutionLog`
- `recompute()` 中に各ノードの再計算時間を `Instant` で計測して記録
- `engine.print_log()` でサマリーを表示

### 完成条件

```
[recompute] filtered    3µs  (dirty)
[recompute] total       1µs  (dirty)
[skip]      summary          (clean)
```

---

## Phase 5: SQLite 永続キャッシュ (1週)

### 実装内容

- `src/cache.rs` — rusqlite で `cache` テーブルを管理
- `recompute()` 後に計算結果を JSON シリアライズして保存
- 起動時に `cache` テーブルから前回の値を復元し clean とみなす

### 完成条件

```bash
# 1回目: 全ノード再計算
cargo run -- --graph examples/pipeline.yaml
# 2回目: キャッシュから復元 (再計算ゼロ)
cargo run -- --graph examples/pipeline.yaml
```

---

## Phase 6: Rayon 並列再計算 (1週)

### 実装内容

- `src/scheduler.rs` — トポロジカル「層」を計算し、同一層内を `rayon::scope` で並列実行
- 同一層 = 互いに依存関係がないノード群

### 完成条件

```bash
cargo bench recompute
# Phase 3 (逐次) vs Phase 6 (並列) を比較
```

---

## 実装順序の根拠

グラフ → 伝播 → 再計算 の順に積み上げることで、各レイヤーを単体テストできる。
Phase 5 (永続化) は Phase 4 のログが整った後に追加することで、
「キャッシュに何が入っているか」を人が確認しやすい状態で実装できる。
