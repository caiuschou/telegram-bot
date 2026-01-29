# 向量搜索准确度优化开发计划 — 评审意见

## 评审概要

| 项目 | 结论 |
|------|------|
| **文档类型** | 开发计划（表格形式） |
| **与 AGENTS.md 符合度** | 符合：有具体开发计划、表格存储、覆盖文档与测试 |
| **与代码现状一致性** | 一致：SyncAIHandler 写死 10/5、BotConfig 无 memory 相关字段、MemoryStore 仅返回条目 |
| **建议** | 采纳前需统一默认值表述、补充 4.2 子页路径约定、明确阶段 2 trait 决策记录位置 |

---

## 一、优点

1. **结构清晰**：项目概述 → 现状 → 与历史方案对应 → 分阶段任务 → 加强项 → 时间表/里程碑/风险/验收，层次完整。
2. **表格驱动**：阶段 1–4 任务均以表格列出 ID、任务、工时、依赖、状态、验收标准，符合 AGENTS.md 对开发计划的要求。
3. **现状与代码一致**：
   - `SyncAIHandler` 中确为 `RecentMessagesStrategy::new(10)`、`SemanticSearchStrategy::new(5, ...)`（见 `ai-handlers/src/sync_ai_handler.rs`）。
   - `BotConfig`（`telegram-bot/src/config.rs`）当前无 `memory_recent_limit`、`memory_relevant_top_k`。
   - `MemoryStore::semantic_search` 仅返回 `Vec<MemoryEntry>`，无分数（见 `crates/memory-core/src/store.rs`）。
4. **风险与依赖显式列出**：MemoryStore trait 变更、Lance API 能力以调研为准，便于排期与沟通。
5. **加强项落地到具体任务**：可观测性（2.7）、回归集（4.4）、成本权衡与异常降级（4.2）、阈值全过滤 warning（2.4/2.6）均有对应任务或验收，可执行性强。
6. **后续项单独成阶段 5**：不占主工期，仅列项，避免范围蔓延。

---

## 二、与现有文档/配置的衔接

1. **docs/rag/configuration.md**  
   已存在 `MEMORY_RECENT_LIMIT=5`、`MEMORY_RELEVANT_TOP_K=3` 的示例与说明。计划中阶段 1.4 为「更新 .env.example 与 docs/rag/configuration.md」，建议在 1.4 验收中注明：以「接入后实际生效的配置」为准统一示例与推荐范围，避免文档与实现脱节。

2. **默认值表述一致**  
   - 阶段 1.1 验收写「有默认值（如 10、5）」  
   - configuration.md 示例为 5、3  
   建议在计划中明确「默认值」的最终取值（例如：recent_limit 默认 10、relevant_top_k 默认 5），并在 1.4 中写清与 configuration.md 的默认/示例一致，避免实现时分歧。

3. **子页路径**  
   阶段 4.2 要求编写 `docs/rag/memory/vector-search-accuracy.md`。当前项目已有 `docs/rag/memory/` 下多篇文档（如 embeddings.md、storage.md）。建议在计划或 README 索引中固定该路径，后续 README 入口链接与 4.1 验收均指向同一路径。

---

## 三、建议与改进

### 3.1 阶段 2：trait 变更决策记录

任务 2.1 要求对「store 层返回 (score, entry) 还是仅在策略层做二次过滤」做决策。建议在验收标准中增加一条：**将决策结论与理由记录在文档中**（例如写在 `docs/rag/memory/vector-search-accuracy.md` 或 `docs/rag/implementation.md` 的「设计决策」小节），便于后续维护与评审。

### 3.2 阶段 1：runner 与 BotComponents 的验收可更具体

任务 1.2 验收为「BotComponents 或构建链能拿到 recent_limit、relevant_top_k」。当前 `BotComponents`（`telegram-bot/src/runner.rs`）不含 memory 相关字段，构建链在 `initialize_bot_components` 中创建 `SyncAIHandler`。建议验收明确为：**BotConfig 的 memory_recent_limit、memory_relevant_top_k 在 runner 中传入 SyncAIHandler（或传入构建 ContextBuilder 的工厂），并在 SyncAIHandler 中用于构造 RecentMessagesStrategy / SemanticSearchStrategy**，便于实现与测试时对「配置下传」有统一理解。

### 3.3 阶段 3：Lance 环境变量命名

任务 3.5 提到「MEMORY_SEMANTIC_FETCH_MULTIPLIER 或调大默认 10/50」。若最终采用新环境变量，建议与现有命名风格统一（如 MEMORY_* 或 LANCE_*），并在 4.2 子页中列入配置项表，避免遗漏文档。

### 3.4 单元测试位置

AGENTS.md 要求「单元测试使用独立的测试文件，不要和代码文件混在一起」。计划中阶段 1.5、2.6 等已要求单元/集成测试，当前项目在 `crates/*/tests/`、`ai-handlers/tests/` 等均有独立测试文件。建议在计划中明确：**新增测试放在各 crate 的 `tests/` 或现有 test 模块中，不放在 `src/*.rs` 内**，与现有规范一致。

### 3.5 验收标准汇总中的「可选」含义

文档末尾验收标准中多处为「可选：…」。建议区分两类含义并写清：  
- **可选功能**：实现与否均可，不影响主验收（如 Lance 高准确度配置）；  
- **可选参数**：参数存在但默认值保持原行为（如 MEMORY_SEMANTIC_MIN_SCORE 默认 0.0）。  
这样发布/验收时不会对「必须实现」产生歧义。

---

## 四、小问题与修正建议

| 位置 | 问题 | 建议 |
|------|------|------|
| 阶段 2.3 依赖 | 写「若 2.1 决定 trait 扩展」：若决策为「仅策略层过滤」则 2.3 可能不做 | 在 2.1 验收或阶段 2 说明中注明：当决策为不改 trait 时，2.3 标记为「不适用」或从本阶段移除，避免任务悬空 |
| 阶段 4.4 | 「3～5 条黄金用例」未规定数据来源（如固定 fixture、种子数据） | 验收中可补充：用例的查询与期望命中的 message_id/内容来自可复现的 fixture 或文档约定，便于 CI 稳定 |
| 整体时间表 | 合计「3–5 天」与「含加强项与可选 4.4 约 4–6 天」中，加强项多数已并入阶段 2/4 任务，不是额外天数 | 可改为：「3–5 天」为主工期；「含可选 4.4 回归集约 +0.5–1 天」，避免读者误以为加强项单独再加 1 天 |

---

## 五、结论

- 文档**符合** AGENTS.md 对开发计划（表格、文档、测试）的要求，与当前代码现状**一致**，阶段划分和加强项落地**可执行**。
- 建议在采纳前：**统一默认值表述与 configuration.md**、**明确 4.2 子页路径与 2.1 决策记录位置**、**细化 1.2 验收与验收汇总中「可选」的含义**；其余为可选改进，可按需纳入计划或实现时遵循。
