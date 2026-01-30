# 向量搜索准确度优化开发计划

## 项目概述

| 项目 | 说明 |
|------|------|
| **项目名称** | 向量搜索准确度优化 |
| **目标** | 提升 RAG 语义检索的准确度与可配置性，减少无关上下文、提高召回可控性 |
| **范围** | 配置接入、相似度过滤、Lance 检索参数、文档与测试；**加强**：可观测性、回归集、成本权衡说明、异常降级、后续混合/重排 |
| **技术栈** | Rust, memory-strategies, memory-lance, telegram-bot config |
| **总工期** | 主工期约 3–5 天；含可选任务 4.4（回归集）约 +0.5–1 天 |

## 现状简述

| 现状项 | 说明 |
|--------|------|
| 近期条数 | `SyncAIHandler` 中写死为 10，配置项 `MEMORY_RECENT_LIMIT` 未接入 |
| 语义条数 | `SyncAIHandler` 中写死为 5，配置项 `MEMORY_RELEVANT_TOP_K` 未接入 |
| 相似度过滤 | 无最低分数阈值，低相关结果也会进入上下文 |
| 距离度量 | Lance 默认 Cosine，与归一化 embedding 匹配 |
| Lance 检索 | 有 user/conversation 过滤时多取 10 倍候选；未暴露 refine/nprobe 等参数 |
| 嵌入模型 | 支持 OpenAI / 智谱，文档建议高精度可选用 text-embedding-3-large |

## 与之前方案的对应关系

| 之前讨论的优化点 | 在计划中的体现 |
|------------------|----------------|
| 接上 MEMORY_RELEVANT_TOP_K，近期条数可配置 | 阶段 1：BotConfig + SyncAIHandler 使用 MEMORY_RECENT_LIMIT、MEMORY_RELEVANT_TOP_K |
| 加相似度阈值，过滤低分结果 | 阶段 2：MemoryStore 返回分数 + SemanticSearchStrategy 支持 MEMORY_SEMANTIC_MIN_SCORE |
| 距离度量保持 Cosine、embedding 归一化 | 现状简述已说明；子页 4.2 可写「推荐保持 Cosine、确认 API 是否已归一化」 |
| 数据量小时不建索引 / 调大 refine、nprobe | 阶段 3：Lance 精确检索或 refine_factor 等可选参数 |
| 调大过滤时的 fetch_limit（user/conversation 过滤后条数不足时） | 阶段 3 补充：Lance 过滤时 fetch_limit 可配置或调大（见 3.5） |
| 换更强的 embedding 模型（如 text-embedding-3-large） | 阶段 4.2 子页写「模型选择与推荐」；若需「通过配置切换模型」见阶段 5 |
| 查询扩展 / 存库粒度 / 时间衰减（后续迭代） | 阶段 5：后续可选优化，仅列项不占主工期 |

## 开发阶段与任务

### 阶段 1：配置接入（Top-K 与相关项）

**目标**：将语义检索条数、近期条数等改为可配置，便于调参优化准确度。  
**工期**：0.5–1 天  
**优先级**：P0

| ID | 任务 | 工时 | 依赖 | 状态 | 验收标准 |
|----|------|------|------|------|----------|
| 1.1 | BotConfig 增加 memory_recent_limit、memory_relevant_top_k | 1h | 无 | 已完成 | 从环境变量 MEMORY_RECENT_LIMIT、MEMORY_RELEVANT_TOP_K 读取；默认值：recent_limit=10、relevant_top_k=5 |
| 1.2 | runner 初始化时将上述配置传入 SyncAIHandler | 0.5h | 1.1 | 已完成 | BotConfig 的 memory_recent_limit、memory_relevant_top_k 在 runner 中传入 SyncAIHandler，并用于构造 ContextBuilder 的 RecentMessagesStrategy / SemanticSearchStrategy |
| 1.3 | SyncAIHandler 使用配置的 recent_limit、relevant_top_k 构建 ContextBuilder | 1h | 1.2 | 已完成 | RecentMessagesStrategy(recent_limit)、SemanticSearchStrategy(relevant_top_k)，不再写死 10/5 |
| 1.4 | 更新 .env.example 与 docs/rag/configuration.md | 0.5h | 1.3 | 已完成 | 文档说明 MEMORY_RECENT_LIMIT、MEMORY_RELEVANT_TOP_K 含义与推荐范围；默认值、示例与 1.1 一致 |
| 1.5 | 为配置项编写单元测试（config 加载、默认值） | 1h | 1.4 | 已完成 | 测试放在 telegram-bot 的 tests/ 或现有 test 模块；覆盖默认值与显式设置，测试通过 |

**阶段 1 小计**：约 4 工时

---

### 阶段 2：相似度阈值过滤

**目标**：对语义检索结果按相似度过滤，低于阈值的不进入上下文，提升准确度观感。  
**工期**：1–1.5 天  
**优先级**：P0  
**说明**：若任务 2.1 决策为**不改 trait**（仅在策略层做二次过滤），则任务 2.3 不适用，可标记为跳过。

| ID | 任务 | 工时 | 依赖 | 状态 | 验收标准 |
|----|------|------|------|------|----------|
| 2.1 | MemoryStore::semantic_search 是否返回分数：评估接口（带分数 vs 仅条目） | 1h | 无 | 已完成 | 决策：在 store 层返回 (score, entry) 或仅在策略层做二次过滤；如改 trait 需兼容 Lance/SQLite/InMemory；**将决策结论与理由记录在 docs/rag/memory/vector-search-accuracy.md 或 implementation 的设计决策小节** |
| 2.2 | LanceVectorStore::semantic_search 返回相似度分数 | 2h | 2.1 | 已完成 | 查询结果带分数（或与现有 nearest_to 结果一致），供策略层或统一过滤使用 |
| 2.3 | SQLiteVectorStore / InMemoryVectorStore 返回相似度分数（仅当 2.1 决定扩展 trait 时执行） | 2h | 2.2 | 已完成 | 与 Lance 行为一致，接口统一；若 2.1 决策为不改 trait 则本任务跳过 |
| 2.4 | SemanticSearchStrategy 支持最小相似度阈值，过滤低分条目 | 1.5h | 2.2 | 已完成 | 可配置 threshold（如 0.0 表示不过滤），只保留 score >= threshold 的条目 |
| 2.5 | BotConfig 增加 memory_semantic_min_score，默认 0.0（不破坏现有行为） | 0.5h | 2.4 | 已完成 | 环境变量 MEMORY_SEMANTIC_MIN_SCORE，解析为 f32，文档说明推荐范围（如 0.6–0.8） |
| 2.6 | 为阈值过滤编写单元/集成测试 | 1.5h | 2.5 | 已完成 | 测试放在 memory-strategies / memory-lance 等 crate 的 tests/；覆盖：低于阈值不返回、高于阈值保留、阈值为 0 时行为与现有一致 |
| 2.7 | 语义检索可观测：打日志（top_k 分数 min/mean/max、命中条数）；全部被阈值过滤时打 warning | 1h | 2.4 | 已完成 | 便于调参与排错，日志可采样或按 debug 级别 |

**阶段 2 小计**：约 9.5 工时

---

### 阶段 3：Lance 检索精度可选优化

**目标**：在保证兼容的前提下，为大数据量场景提供“更准”的选项（如不建索引或调大 refine 类参数）。  
**工期**：1–2 天  
**优先级**：P1

| ID | 任务 | 工时 | 依赖 | 状态 | 验收标准 |
|----|------|------|------|------|----------|
| 3.1 | 调研 Lance 当前 API：refine_factor / nprobe / 精确检索选项 | 2h | 无 | 已完成 | 文档或代码注释记录：如何提高精度、是否支持仅暴力搜索 |
| 3.2 | LanceConfig 增加可选参数（如 use_exact_search 或 refine_factor） | 1h | 3.1 | 已完成 | 配置项有默认值，不改变现有调用方行为 |
| 3.3 | LanceVectorStore::semantic_search 根据配置选择检索方式 | 2h | 3.2 | 已完成 | 小数据量或明确要求时可用精确检索；否则保持现有逻辑 |
| 3.4 | 文档更新：LANCE_USAGE.md、configuration 说明何时追求准确度、何时追求速度 | 1h | 3.3 | 已完成 | 用户能根据场景选择“高准确度”或“高性能” |
| 3.5 | Lance 过滤时 fetch_limit 可配置或调大（user_id/conversation_id 过滤后条数不足时） | 1h | 3.1 | 已完成 | 如新增环境变量（如 MEMORY_SEMANTIC_FETCH_MULTIPLIER 或 LANCE_*），需在阶段 4.2 子页配置项表中列出；文档说明适用场景 |

**阶段 3 小计**：约 7 工时

---

### 阶段 4：文档与 CHANGELOG

**目标**：索引与子页完整，变更可追溯。  
**工期**：0.5 天  
**优先级**：P1

| ID | 任务 | 工时 | 依赖 | 状态 | 验收标准 |
|----|------|------|------|------|----------|
| 4.1 | docs/rag/README.md 增加“向量搜索准确度优化”入口链接 | 0.5h | 阶段 1–3 | 已完成 | 索引可链到本开发计划及 configuration |
| 4.2 | 编写 docs/rag/memory/vector-search-accuracy.md：优化手段汇总、配置项、推荐值；含「成本与准确度权衡」表、「异常与降级」、embedding 模型建议 | 2.5h | 4.1 | 已完成 | 子页路径固定为 docs/rag/memory/vector-search-accuracy.md；含 Top-K、阈值、距离度量、索引选择及阶段 3 新增的 Lance/内存相关环境变量；高准确度/平衡/高速度三档推荐配置；embedding 失败与检索为空的降级说明 |
| 4.3 | 在 CHANGELOGS.md 中记录“向量搜索准确度优化”相关变更 | 0.5h | 4.2 | 已完成 | 按版本或日期记录新增配置与行为变化 |
| 4.4 | （可选）建立语义检索回归集：3～5 条黄金用例（查询→期望命中），集成测试或脚本断言召回包含期望 | 2h | 2.6 | 已完成 | 用例的查询与期望命中的 message_id/内容来自可复现的 fixture 或文档约定，便于 CI 稳定；可选 P1 |

**阶段 4 小计**：约 5～7 工时（含 4.4 则+2h）

---

### 阶段 5：后续可选优化（不占主工期）

**目标**：将之前讨论的其余优化点列为可选/后续项，便于迭代时选用。  
**优先级**：P2

| ID | 优化点 | 说明 |
|----|--------|------|
| 5.1 | Embedding 模型通过配置切换 | 若需在运行时切换 text-embedding-3-small / text-embedding-3-large 或智谱模型，可增加 EMBEDDING_MODEL 等配置并接到现有 Embedding 服务；换模型需注意与 Lance/SQLite 的 embedding_dim 一致 |
| 5.2 | 查询扩展 | 用大模型把用户问题改写成 1～2 个同义问句，多向量检索或取并集再排序，提高召回 |
| 5.3 | 存库粒度 | 长消息按句/按段切分后再做 embedding 写入，检索粒度更细，准确度更好 |
| 5.4 | 时间衰减 | 对 semantic_search 返回的分数按时间衰减（如 docs/rag/future.md 中的 decay_score），旧记忆权重降低 |
| 5.5 | 混合检索（关键词 + 向量） | 短查询或专有名词同时做关键词匹配与向量检索，结果合并去重或按分数融合，提高召回 |
| 5.6 | 二阶段重排序 | 向量召回较多候选后，用轻量模型或规则重排再取 top_k，提高精度 |

以上仅作规划项，不做工时与状态跟踪；实现时再拆分为具体任务。

---

## 方案加强项

在现有阶段基础上，从「可衡量、可观测、可回归、可权衡」四方面加强，使方案更可执行、效果可验证。

### 1. 准确度评估与可观测性

| 加强点 | 说明 | 建议落地 |
|--------|------|----------|
| 相似度分布可观测 | 调参（top_k、阈值）需要看到真实分数分布，否则阈值难以设定 | 阶段 2 完成后：在 SemanticSearchStrategy 或 store 层对每次 semantic_search 打日志（如 top_k 条分数 min/mean/max、命中条数），可选采样率避免刷屏 |
| 检索结果摘要日志 | 便于排查「为什么没召回某条」：当前 query、limit、过滤条件、返回条数、是否被阈值截断 | 与上同点一并加入，或写在 4.2 子页「运维与排错」 |
| 人工抽检 / 标注（可选） | 若有少量典型对话，可定期抽检「问题 → 应召回的片段」是否被检索到，用于定性评估 | 阶段 4.2 子页写「可选：人工抽检清单与频率」，不占主工期 |

### 2. 回归测试与黄金集

| 加强点 | 说明 | 建议落地 |
|--------|------|----------|
| 语义检索回归集 | 固定若干「查询 + 期望命中的 message_id 或内容片段」，每次改 store/策略/配置后跑一遍，防止准确度回退 | 新增任务 4.4：建立 3～5 条黄金用例（如「猫」→ 含「猫」的某条记忆），集成测试或单独脚本断言召回列表包含期望；可选 P1 |
| 阈值与 top_k 回归 | 修改默认阈值或 top_k 时，回归集仍通过或预期变更在文档中说明 | 与上同，用例覆盖「默认配置」「高阈值」「大 top_k」等组合 |

### 3. 成本与准确度权衡

| 加强点 | 说明 | 建议落地 |
|--------|------|----------|
| 推荐配置表 | 不同场景（开发/生产、小数据/大数据、要准/要快）给出推荐配置组合，避免盲目调参 | 阶段 4.2 子页增加小节「成本与准确度权衡」：表格列出「高准确度」「平衡」「高速度」三档的 top_k、阈值、是否精确检索、embedding 模型建议 |
| 换模型成本提示 | 使用 text-embedding-3-large 或智谱时，标注相对成本与维度变更注意点 | 同上，写在「模型选择与推荐」 |

### 4. 异常与降级

| 加强点 | 说明 | 建议落地 |
|--------|------|----------|
| 统一降级策略 | embedding 失败或 semantic_search 返回空时，当前已有「跳过语义、仅用近期」等行为，在文档和代码注释中写清，便于维护和扩展 | 阶段 4.2 子页写「异常与降级」：embedding 失败 / 检索为空 / 全被阈值过滤时的行为；可选在 SemanticSearchStrategy 注释中引用该文档 |
| 阈值过高导致常为空 | 若用户将 MEMORY_SEMANTIC_MIN_SCORE 设得过高，可能导致语义块常为空，可在日志中打 warning（如「semantic 结果全部低于阈值」） | 阶段 2.4 或 2.6 验收中补充：当全部被阈值过滤时打一条 warning 日志 |

### 5. 后续可选：混合检索与重排序

| 加强点 | 说明 | 建议落地 |
|--------|------|----------|
| 关键词 + 向量混合 | 对短查询或专有名词，可同时做关键词匹配（如 SQLite LIKE 或简单 token 匹配）与向量检索，结果合并去重或按分数融合 | 阶段 5 后续项补充 5.5：混合检索（关键词 + 向量），实现时再拆任务 |
| 二阶段重排序 | 用向量检索召回较多候选后，再用轻量模型（如 cross-encoder 或规则）对候选重排，取 top_k，提高精度 | 阶段 5 后续项补充 5.6：二阶段重排序，实现时再拆任务 |

---

## 整体时间表

| 阶段 | 内容 | 工期 |
|------|------|------|
| 阶段 1 | 配置接入（Top-K 等） | 0.5–1 天 |
| 阶段 2 | 相似度阈值过滤 | 1–1.5 天 |
| 阶段 3 | Lance 检索精度可选优化 | 1–2 天 |
| 阶段 4 | 文档与 CHANGELOG（含成本权衡、异常降级；可选回归集） | 0.5–1 天 |

**合计**：主工期约 3–5 天（1 人日 ≈ 6–8 工时）；含可选任务 4.4（回归集）约 +0.5–1 天。

## 里程碑

| 里程碑 | 完成标准 |
|--------|----------|
| M1: 配置接入完成 | MEMORY_RELEVANT_TOP_K、MEMORY_RECENT_LIMIT 生效，SyncAIHandler 使用配置值，测试通过 |
| M2: 阈值过滤完成 | 语义检索支持最小相似度过滤，默认 0 不改变现有行为，测试通过 |
| M3: Lance 可选优化完成 | 配置与文档就绪，用户可选用高精度检索选项 |
| M4: 文档与发布就绪 | README 索引、accuracy 子页（含成本权衡、异常降级）、CHANGELOG 更新完成；语义检索可观测日志与阈值全过滤 warning 已落地；可选 M5：回归集通过 |

## 风险与依赖

| 风险/依赖 | 说明 |
|-----------|------|
| MemoryStore trait 变更 | 若 semantic_search 改为返回带分数结果，需所有实现（Lance/SQLite/InMemory）同步修改并跑通集成测试 |
| Lance API 能力 | refine_factor/nprobe 等是否暴露、默认值需以当前 lancedb 版本为准，阶段 3 以调研结果为准 |

## 验收标准汇总

### 必须达成

- 语义检索条数由 `MEMORY_RELEVANT_TOP_K` 控制，近期条数由 `MEMORY_RECENT_LIMIT` 控制。
- 配置与优化手段在 docs/rag 下有完整索引与子页（含 embedding 模型建议、距离度量、索引选择），CHANGELOG 已更新。
- **加强**：语义检索打日志（分数分布、命中条数），全部被阈值过滤时打 warning；子页 `docs/rag/memory/vector-search-accuracy.md` 含「成本与准确度权衡」表与「异常与降级」说明。

### 可选参数（实现必做，默认不改变现有行为）

- 语义检索结果按 `MEMORY_SEMANTIC_MIN_SCORE` 过滤，**默认 0.0** 保持原行为。

### 可选功能（可不做，不影响主验收）

- Lance 支持“高准确度”配置（如精确检索或更大 refine），文档说明使用场景。
- Lance 在 user/conversation 过滤时 fetch_limit 可配置或调大，避免过滤后条数不足。
- 回归集（任务 4.4）：3～5 条黄金用例防止准确度回退。
- 后续（阶段 5）：embedding 模型配置切换、查询扩展、存库粒度、时间衰减、混合检索、二阶段重排序，按需迭代。

## 评审结论（摘要）

- 计划符合 AGENTS.md（表格、文档、测试），与代码现状一致，可执行。
- 采纳建议：默认值与 configuration.md 统一；4.2 子页路径固定为 `docs/rag/memory/vector-search-accuracy.md`；阶段 2 trait 决策记录于该子页或 implementation 设计决策；验收区分「可选参数」（默认保持原行为）与「可选功能」（可不做）。
