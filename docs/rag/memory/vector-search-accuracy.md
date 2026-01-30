# 向量搜索准确度

本文档描述 RAG 语义检索的准确度优化手段、配置项、推荐值，以及成本权衡与异常降级。

## 设计决策：语义检索是否返回分数

**决策**：在 **store 层** 让 `MemoryStore::semantic_search` 返回带相似度分数的结果 `(score, entry)`，策略层再按阈值过滤与打日志。

**理由**：

- 分数由各存储实现计算（Lance 的 `_distance`、SQLite/InMemory 的余弦相似度），在 store 层已有信息，统一返回可避免重复计算。
- 策略层需要分数才能做阈值过滤与可观测日志（min/mean/max、全被过滤时 warning），若 trait 不返回分数，策略层无法实现。
- 扩展 trait 后，Lance / SQLite / InMemory 三种实现统一接口，调用方（如 `SemanticSearchStrategy`）只需一处逻辑：按 `MEMORY_SEMANTIC_MIN_SCORE` 过滤并打日志。
- 对现有调用方：策略层过滤后仍得到 `Vec<MemoryEntry>` 用于构建上下文，对外行为不变；默认阈值 0.0 表示不过滤，保持兼容。

**结论**：扩展 `MemoryStore::semantic_search` 返回类型为 `Vec<(f32, MemoryEntry)>`，其中 `f32` 为相似度分数（越高越相似，如余弦相似度）。各 store 实现必须返回分数；策略层负责按阈值过滤与可观测性。

---

## 配置项与推荐值

| 配置项 | 含义 | 默认值 | 推荐范围 |
|--------|------|--------|----------|
| `MEMORY_RELEVANT_TOP_K` | 语义检索返回的最大条数 | 5 | 3–20，按上下文长度与成本权衡 |
| `MEMORY_RECENT_LIMIT` | 近期消息条数 | 10 | 5–30 |
| `MEMORY_SEMANTIC_MIN_SCORE` | 语义检索最低相似度阈值，低于此分数的条目不进入上下文 | 0.0 | 0.6–0.8 可减少无关上下文；0.0 表示不过滤 |

详见 [configuration.md](../configuration.md)。

---

## 成本与准确度权衡

| 场景 | Top-K | 阈值 | 检索方式（Lance） | Embedding 模型建议 |
|------|-------|------|-------------------|---------------------|
| 高准确度 | 10–15 | 0.7–0.8 | 精确检索或更大 refine | text-embedding-3-large 等 |
| 平衡 | 5–10 | 0.6–0.7 | 默认索引 | text-embedding-3-small / 智谱 embedding-2 |
| 高速度 | 3–5 | 0.0–0.5 | 默认索引、小 nprobe | 小维度模型 |

### Lance 检索参数（LanceConfig）

创建 `LanceVectorStore` 时可通过 `LanceConfig` 调节检索精度与速度（默认不改变现有行为）：

| 参数 | 含义 | 默认值 | 适用场景 |
|------|------|--------|----------|
| `use_exact_search` | 为 true 时跳过向量索引（暴力搜索） | false | 小/中表追求最高准确度 |
| `refine_factor` | IVF-PQ 精排：取 limit×refine_factor 候选再重算距离 | None | 有 IVF-PQ 索引时提高排序精度 |
| `nprobes` | IVF 搜索分区数 | None（Lance 默认 20） | 提高召回、可接受延迟时增大 |
| `semantic_fetch_multiplier` | 按 user/conversation 过滤时 fetch_limit = limit × 此值（至少 50） | 10 | 过滤后条数不足时可调大 |

### 距离度量与索引选择

- **距离度量**：与归一化 embedding 配合时推荐保持 **Cosine**（LanceConfig.distance_type）；若换模型需确认 API 是否已归一化。
- **索引选择**：无索引或小表可设 `use_exact_search=true` 做暴力最近邻；有 IVF-PQ 索引时可设 `refine_factor` 提高排序精度；有 IVF 索引时可设 `nprobes` 提高召回。
- **换模型成本**：使用 text-embedding-3-large 或智谱时，注意相对成本与 **embedding 维度**（需与 Lance/SQLite 表结构一致，否则需重建或迁移）。

### Embedding 模型建议

| 场景 | 建议模型 |
|------|----------|
| 高准确度 | OpenAI text-embedding-3-large、智谱更高维模型等 |
| 平衡 | OpenAI text-embedding-3-small、智谱 embedding-2 |
| 高速度 / 低成本 | 小维度模型（注意与现有表 dimension 一致） |

---

## 异常与降级

| 情况 | 行为 |
|------|------|
| Embedding 失败 | 跳过语义检索，仅用近期消息与用户偏好构建上下文；打 warning 日志。 |
| semantic_search 返回空 | 不报错，语义块为空，其余策略（近期、偏好）照常。 |
| 全部被阈值过滤 | 仅保留 score ≥ `MEMORY_SEMANTIC_MIN_SCORE` 的条目；若全部低于阈值则语义块为空，并打 **warning**（如「semantic 结果全部低于阈值」）便于调参。 |

策略层在 `SemanticSearchStrategy` 中实现上述逻辑；代码注释可引用本文档。

---

## 可观测性

- **语义检索日志**：每次 semantic_search 后打日志，包含 top_k 条分数的 **min/mean/max**、命中条数（过滤前/后）。
- **全部被阈值过滤**：打一条 warning，便于发现阈值过高或 query 与库中内容不匹配。

---

## 语义检索回归集（黄金用例）

为防止准确度回退，项目在 **memory-lance** 中维护可复现的语义检索回归集（不依赖外部 API，CI 稳定）：

- **测试**：`memory-lance/tests/lance_vector_store_integration_test.rs` 中的 `test_semantic_search_regression_golden_cases`。
- **Fixture**：三条 MemoryEntry（A/B/C），embedding 分别为 1536 维 one-hot（第 0/1/2 维为 1，其余为 0），content 为 "entry A" / "entry B" / "entry C"。
- **黄金用例**：查询向量与某条 embedding 一致时，该条应为首条召回。

| 用例 | 查询向量 | 期望命中内容 |
|------|----------|--------------|
| 1 | 第 0 维为 1，其余 0 | "entry A" |
| 2 | 第 1 维为 1，其余 0 | "entry B" |
| 3 | 第 2 维为 1，其余 0 | "entry C" |

修改默认阈值、top_k 或 Lance 检索逻辑后，运行 `cargo test -p memory-lance test_semantic_search_regression_golden_cases` 可验证回归集仍通过。

---

## 相关文档

- [向量搜索准确度优化开发计划](../vector-search-accuracy-plan.md)
- [configuration.md](../configuration.md)
- [storage.md](./storage.md)（MemoryStore 与各实现）
- [embeddings.md](./embeddings.md)（Embedding 模型与维度）
