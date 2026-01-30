# CLI 查询向量数据库最近 N 条记录 - 方案

## 1. 目标

在 `dbot-cli` 中增加一个子命令，用于查询 LanceDB 向量数据库中**按时间倒序的最近 N 条记录**（默认 100 条），便于运维排查、数据抽查和调试。

## 2. 现状与约束

| 项目 | 说明 |
|------|------|
| CLI 现状 | `dbot-cli` 已有 `run`、`load` 子命令；`load` 从 SQLite 加载消息到 LanceDB，依赖 `memory-loader`、`LANCE_DB_PATH`、`embedding` 配置。 |
| 向量存储 | `memory-lance::LanceVectorStore` 实现 `MemoryStore`，表结构含 `id, content, vector, user_id, conversation_id, role, timestamp, tokens, importance`。 |
| MemoryStore trait | `memory-core::MemoryStore` 仅有 `add/get/update/delete/search_by_user/search_by_conversation/semantic_search`，**没有「按时间倒序取 N 条」的接口**。 |
| LanceDB Rust API | `lancedb` 0.23 的 `QueryBase` 提供 `limit`、`offset`、`only_if` 等，**未提供 `order_by`**；全表扫描后需在内存中按 `timestamp` 排序。 |

## 3. 方案概述

- **CLI**：新增子命令 `dbot list-vectors`（或 `vectors recent`），可选参数 `--limit 100`、`--lance-db-path`（覆盖 `LANCE_DB_PATH`）。
- **数据来源**：直接使用 **LanceDB**（与 `load` 一致），不依赖 SQLite；只读查询，不写库。
- **「最近」语义**：按 `MemoryEntry.metadata.timestamp` **降序**，取前 N 条。
- **实现层级**：在 **memory-lance** 中为 `LanceVectorStore` 增加**非 trait 方法** `list_recent(limit: usize)`，CLI 依赖 `memory-lance` 并调用该方法；**不扩展** `MemoryStore` trait，避免所有实现类（inmemory、sqlite、lance）都实现一遍。

## 4. 开发计划（表格）

| 序号 | 任务 | 说明 | 产出 |
|------|------|------|------|
| 1 | memory-lance：实现 `list_recent(limit)` | 在 `LanceVectorStore` 上新增方法：打开表 → `table.query().limit(max_fetch).execute()`（见下）→ 将每行转为 `MemoryEntry` → 按 `timestamp` 降序排序 → 取前 `limit` 条返回。`max_fetch` 可设为 `limit` 的倍数（如 10 倍）或上限（如 10000），避免全表拉取。 | `memory-lance/src/store.rs` 新增方法及注释；必要时 `lib.rs` 导出。 |
| 2 | memory-lance：list_recent 行为与边界 | 空表返回空 vec；`limit=0` 返回空 vec；不依赖 embedding 维度（只读已有列）。若 Lance 表不存在或未初始化，与现有 `ensure_table` 行为一致（首次查询时表存在即可）。 | 同上；可选：在现有 integration test 里加「list_recent 返回条数、顺序」断言。 |
| 3 | dbot-cli：新增子命令与参数 | 子命令名：`list-vectors`（或 `vectors recent`）。参数：`--limit` 默认 100；`--lance-db-path` 可选，默认从 `LANCE_DB_PATH` 读取；与 `load` 类似，需 `embedding_dim` 以打开 Lance（可从 embedding 配置推导，或从环境变量读取固定维度）。 | `dbot-cli/src/main.rs`：新 enum 分支、参数解析、调用 list_recent。 |
| 4 | dbot-cli：Lance 连接与维度 | 与 `load` 一致使用 `LANCE_DB_PATH`；Lance 连接需 `embedding_dim`，可从当前 `LoadConfig`/embedding 配置推导（memory-loader 中 `embedding_dim_for_config`），或新增环境变量 `LANCE_EMBEDDING_DIM`（与 load 使用的维度一致）。 | 复用或抽取 `embedding_dim_for_config`；或 dbot-cli 内读 `LANCE_EMBEDDING_DIM`，默认 1536。 |
| 5 | dbot-cli：输出格式 | 控制台打印最近 N 条：每行一条简要信息（如 `id, timestamp, role, user_id, content_preview`），或表格形式；内容过长可截断（如 content 前 80 字符）。 | 打印逻辑在 `handle_list_vectors` 或等价函数中。 |
| 6 | 文档与 CHANGELOG | README 中增加「查询向量库最近记录」用法；CHANGELOGS.md 增加一条记录。 | `dbot-cli/README.md`；`CHANGELOGS.md`。 |
| 7 | 单元/集成测试 | memory-lance：对 `list_recent` 的集成测试（写入若干条不同 timestamp，再 list_recent(2) 断言顺序与数量）。dbot-cli：可选，对 `list-vectors` 做端到端测试（依赖本地 Lance 或临时目录）。 | `memory-lance/tests/` 或现有 integration test；可选 `dbot-cli` 测试。 |

## 5. 技术细节

### 5.1 list_recent 实现要点（memory-lance）

- 使用现有 `batch_to_entry` 将 `RecordBatch` 行转为 `MemoryEntry`。
- 全表或有限范围：`table.query().limit(max_fetch).execute()`，其中 `max_fetch = limit.saturating_mul(10).min(10000)`，避免单次拉取过大；若表总行数小于 `max_fetch`，则当前即「全部」；若大于，则得到的是「表中某子集」按时间取前 N 条（注意：Lance 无 order_by，所以是「先取 max_fetch 条未定义顺序，再在内存按 timestamp 降序取 limit」）。若需严格全局最近 N 条，只能全表扫描后排序（表很大时可在文档中说明性能影响）。
- **推荐**：先实现「全表 scan + 内存按 timestamp 降序 + take(limit)」以保证语义正确；若表行数过多再考虑「流式/分页 + 堆」等优化。

### 5.2 CLI 子命令示例

```text
dbot list-vectors [--limit 100] [--lance-db-path PATH]
```

- 从 `.env` 读 `LANCE_DB_PATH`（缺省 `./lancedb`）、`LANCE_EMBEDDING_DIM` 或与 load 一致的 embedding 维度。
- 调用 `LanceVectorStore::with_config(...).await` 后调用 `list_recent(limit)`，再格式化输出。

### 5.3 依赖关系

- **dbot-cli** 已依赖 `memory-loader`（含 embedding 配置）；需**直接依赖** `memory-lance` 以使用 `LanceVectorStore::list_recent`；`memory` 通过 telegram-bot 等已间接存在，若需 `MemoryEntry` 等类型可从 `memory` 或 `memory-lance` 的 re-export 使用。

## 6. 风险与取舍

| 风险 | 缓解 |
|------|------|
| Lance 表很大时全表 scan 慢 | 先实现正确语义（全表 + 排序）；文档注明「适用于中小规模向量表」；后续可加 `--max-scan` 或服务端 order_by（若 Lance 未来支持）。 |
| embedding_dim 与 load 不一致 | 与 load 共用同一套配置或同一环境变量，并在 README 中说明。 |
| 不扩展 MemoryStore | 仅 Lance 提供「最近 N 条」；其他 store 不实现，满足「CLI 查 Lance」即可。 |

## 7. 验收标准

- 运行 `dbot list-vectors --limit 100` 能输出当前 Lance 库中按时间倒序的最近 100 条记录（或全部若不足 100 条）。
- 输出包含 id、时间、角色、user_id、内容摘要等可读信息。
- 空库时输出空列表或友好提示；`--lance-db-path` 可覆盖默认路径。
- memory-lance 有 list_recent 的集成测试；CHANGELOG 与 README 已更新。

---

以上为「CLI 查询向量数据库最近 100 条记录」的完整方案，可按表格中的序号依次实现。
