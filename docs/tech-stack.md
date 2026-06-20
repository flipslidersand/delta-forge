# Tech Stack — DeltaForge

## 言語・バージョン

- Rust 1.78+ (edition 2021)

## 主要クレートと選定理由

| クレート     | バージョン | 役割                            | 選定理由                                     |
| ------------ | ---------- | ------------------------------- | -------------------------------------------- |
| `petgraph`   | 0.6        | 有向グラフ + トポロジカルソート | Rust 最定番の汎用グラフライブラリ            |
| `rusqlite`   | 0.31       | SQLite 永続キャッシュ (Phase 5) | 外部サーバー不要・bundled feature で依存最小 |
| `rayon`      | 1          | 並列再計算 (Phase 6)            | データ並列の自然な API                       |
| `serde`      | 1          | ノード値のシリアライズ          | JSON/bincode へのゲートウェイ                |
| `serde_json` | 1          | デバッグ用ログ出力              | 人が読める実行ログ                           |
| `anyhow`     | 1          | エラーハンドリング              | プロトタイプ規模では十分                     |
| `criterion`  | 0.5        | 再計算速度のベンチマーク        | 統計的に正確な計測                           |

## アーキテクチャ

```
DeltaForge (engine)
  ├── Graph (petgraph DiGraph)
  │     ├── InputNode  { value, dirty }
  │     └── ComputeNode { func, cache, dirty }
  ├── Scheduler
  │     ├── propagate_dirty()   — 変更ノードから downstream へ dirty 伝播
  │     └── topo_recompute()    — トポロジカル順に dirty ノードを再計算
  ├── ExecutionLog              — 再計算イベントの記録
  └── Cache (Phase 5)           — SQLite によるキャッシュ永続化
```

## 開発ツール

| ツール      | 用途         |
| ----------- | ------------ |
| `clippy`    | linting      |
| `rustfmt`   | フォーマット |
| `criterion` | ベンチマーク |
