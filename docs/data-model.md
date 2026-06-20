# Data Model — DeltaForge

## コアデータ構造

```rust
pub type NodeId = petgraph::graph::NodeIndex;

/// ノードの種別
pub enum NodeKind {
    /// 外部から値を注入する入力ノード
    Input { value: Box<dyn Any + Send> },
    /// upstream の出力を受け取って計算するノード
    Compute {
        func: Box<dyn Fn(&[&dyn Any]) -> Box<dyn Any + Send> + Send + Sync>,
        cache: Option<Box<dyn Any + Send>>,
    },
}

pub struct Node {
    pub id: NodeId,
    pub name: String,
    pub kind: NodeKind,
    pub dirty: bool,
}

/// エンジン本体
pub struct DeltaForge {
    graph: petgraph::Graph<Node, ()>,  // 有向グラフ (edge: A → B = B は A に依存)
    name_index: HashMap<String, NodeId>,
    log: ExecutionLog,
}
```

## 実行ログ

```rust
pub struct LogEntry {
    pub node_name: String,
    pub recomputed_at: std::time::Instant,
    pub duration_us: u64,
}

pub struct ExecutionLog {
    pub entries: Vec<LogEntry>,
}
```

## SQLite キャッシュ (Phase 5)

```sql
CREATE TABLE cache (
    node_name TEXT PRIMARY KEY,
    value_json TEXT NOT NULL,
    computed_at INTEGER NOT NULL   -- UNIX timestamp
);
```

## 状態遷移

```
ノード状態: Clean ←→ Dirty

update("input_node", new_val)
  → InputNode.value = new_val
  → InputNode.dirty = true
  → downstream を BFS/DFS で Dirty に伝播

recompute()
  → トポロジカルソートでノードを順序付け
  → dirty なノードを順番に再計算
  → ComputeNode.cache を更新、dirty = false
  → LogEntry を追記
```

## 依存グラフの例

```
sales (Input)
  └─▶ filtered (Compute: filter active)
        └─▶ monthly_total (Compute: group+sum)

update("sales", ...) → filtered.dirty, monthly_total.dirty
recompute()          → filtered を先に計算 → monthly_total を計算
```
