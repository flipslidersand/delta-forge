# ADR-004: 永続キャッシュに SQLite (rusqlite) を使う

- **日付**: 2026-06-20
- **状態**: Accepted

## 背景

Phase 5 でキャッシュをプロセス再起動後も維持したい。
RocksDB・redb・SQLite の選択肢がある。

## 決定

`rusqlite` (bundled feature) を使う。

## 理由

- `bundled` feature で libsqlite3 を静的リンクでき、外部インストール不要
- 値を JSON 文字列として保存すれば、スキーマ変更コストが低い
- `rusqlite` は Rust から最も使われている SQLite バインディングで実績十分
- RocksDB はバイナリサイズ・ビルド時間の面でオーバースペック

## トレードオフ

- 大量ノード（数万〜数十万）では RocksDB の方が書き込みスループットが高い
  → MVP 規模（数百ノード）では SQLite で十分
