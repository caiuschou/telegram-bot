# 数据库查询结果

查询时间：按你当前环境实际执行结果整理。

## 1. 项目内找到的数据库文件

| 路径 | 表 | 说明 |
|------|----|------|
| `memory/memory.db` | memory_entries | 测试数据（user123, Hello world），无 conversation_id |
| `crates/memory-sqlite/memory.db` | memory_entries | 测试数据（1 条） |
| `telegram_bot.db` | messages | **消息持久化**，460 条，含真实对话 |
| `target/release/telegram_bot.db` | messages | 同上（发布目录副本） |

**未找到**：`./data/memory.db`（.env 里 `MEMORY_SQLITE_PATH=./data/memory.db`）。  
AI 的「Recent Message」在 `MEMORY_RECENT_USE_SQLITE=1` 时来自 **memory_entries**，若用默认配置则会写/读该路径；当前仓库下没有这个文件，可能是运行目录不同或尚未在该路径创建过。

---

## 2. memory_entries 查询结果（最近消息逻辑用的表）

### 2.1 `memory/memory.db`

```text
conversation_id  cnt
---------------  ---
                 9  

id                                    conversation_id  user_id  role  timestamp                         content_preview
------------------------------------  ---------------  -------  ----  --------------------------------  ---------------
4a1361b3-03c9-4a72-a8a9-5435e519cfa9                   user123  User  2026-01-27T07:15:19.078576+00:00  Hello world
5732a924-294f-420d-925f-3255f321b4ac                   user123  User  2026-01-27T07:17:37.544869+00:00  Hello world
...（共 9 条，均为 User, "Hello world", conversation_id 为空）
```

### 2.2 `crates/memory-sqlite/memory.db`

```text
conversation_id  cnt
---------------  ---
                 1  

id                                    conversation_id  user_id  role  timestamp                         content_preview
------------------------------------  ---------------  -------  ----  --------------------------------  ---------------
ca0f66f3-c7b3-46ec-aef6-a6f17c8765b9                   user123  User  2026-01-28T02:24:36.176510+00:00  Hello world
```

以上均为测试数据，**不是**你 AI 请求里那种「您好！有什么可以帮助您的吗？」、@DbCaiusBot、hello 等对话。

---

## 3. messages 查询结果（消息持久化表，telegram_bot.db）

与请求里「Conversation (recent)」可能对应的是**私聊** `chat_id = -5189538420`。

**说明**：数据库查「最近消息」时用的是**降序**（`ORDER BY created_at DESC` / `ORDER BY timestamp DESC`），新的在前；策略层会再按时间升序排好后取最后 N 条，发给 AI 的「Conversation (recent)」块是**旧→新**。下表按 **DESC**（与 DB 查询一致，第一行是最新一条）。

| created_at | direction | message_type | content（前 90 字） |
|------------|-----------|--------------|----------------------|
| 2026-01-30T08:10:58 | sent | ai_response | hello |
| 2026-01-30T08:10:54 | received | text | @DbCaiusBot hello |
| 2026-01-30T07:43:31 | sent | ai_response | hello |
| 2026-01-30T07:43:27 | received | text | @DbCaiusBot hello |
| 2026-01-30T07:43:22 | received | text | @DbCaiusBot |
| 2026-01-30T07:43:20 | received | text | 6 |
| 2026-01-30T07:43:20 | received | text | 5 |
| 2026-01-30T07:43:19 | received | text | 4 |
| 2026-01-30T07:43:18 | received | text | 3 |
| 2026-01-30T07:43:17 | received | text | 2 |
| 2026-01-30T07:43:17 | received | text | 1 |
| 2026-01-30T05:22:21 | sent | ai_response | 你好！有什么可以帮助你的吗？ |
| 2026-01-30T05:22:08 | received | text | @DbCaiusBot hello |
| 2026-01-30T04:42:01 | received | text | 2 |
| 2026-01-30T04:41:59 | sent | ai_response | 您好！我是基于GLM-4-Flash模型构建的AI助手... |
| 2026-01-30T04:41:57 | received | text | 你是什么模型？ |
| 2026-01-30T04:41:51 | sent | ai_response | 您好！您似乎在测试某个功能或命令... |
| 2026-01-30T04:41:46 | received | text | @DbCaiusBot test |
| 2026-01-30T04:03:10 | sent | ai_response | 您好！根据之前的对话内容，您提到喜欢吃面。... |
| 2026-01-30T04:03:08 | received | text | @DbCaiusBot 我喜欢吃什么？ |
| 2026-01-30T04:03:00 | sent | ai_response | 你好！有什么可以帮助你的吗？ |
| 2026-01-30T04:03:00 | sent | ai_response | 你好！有什么可以帮助你的吗？ |
| 2026-01-30T04:02:59 | sent | ai_response | 您好！有什么可以帮助您的吗？... |
| 2026-01-30T04:02:57 | received | text | @DbCaiusBot hello |
| … | … | … | … |
| 2026-01-30T03:57:55 | received | text | # OpenAI API Configuration... |

---

## 4. 和 AI 请求里的「Conversation (recent)」对照

- AI 请求里 recent 的顺序（节选）：  
  Assistant: 您好！有什么可以帮助您的吗？→ Assistant: 你好！... → Assistant: 你好！... → User: @DbCaiusBot 我喜欢吃什么？→ User: @DbCaiusBot test → Assistant: 您好！您似乎在测试... → User: 你是什么模型？→ User: 2 → User: @DbCaiusBot hello → Assistant: 你好！... → User: 1～6 → User: @DbCaiusBot → User: @DbCaiusBot hello → Assistant: hello → User: @DbCaiusBot hello。

- **messages 表**（上表）里，同一对话的内容和顺序与上面**大体一致**：  
  有「您好！有什么可以帮助您的吗？」、「你好！有什么可以帮助你的吗？」、@DbCaiusBot 我喜欢吃什么、test、你是什么模型、2、@DbCaiusBot hello、1～6、@DbCaiusBot、hello 等，且时间顺序一致。

- **注意**：  
  - 「Recent Message」在代码里来自 **memory_entries**（`MEMORY_SQLITE_PATH` 的 SQLite），不是 **messages** 表。  
  - 当前仓库下没有 `./data/memory.db`，所以无法直接查「真正给 AI 用的」memory_entries。  
  - 但从 **messages** 表可以确认：**数据库里确实有这段对话，且内容和时间顺序与 AI 请求中的 recent 一致**，说明请求里的 recent 与真实持久化对话是对得上的。

---

## 5. 如何自己再查一遍

在项目根或 telegram-bot 目录执行（按你实际 DB 路径改）：

```bash
# 消息持久化：按会话查（与 message_repo 一致，降序，新的在前）
sqlite3 telegram_bot.db "SELECT created_at, direction, message_type, content FROM messages WHERE chat_id = -5189538420 ORDER BY created_at DESC;"

# 若存在「最近消息」用的 SQLite（与 memory-sqlite search_by_conversation 一致，降序）
sqlite3 ./data/memory.db "SELECT timestamp, role, content FROM memory_entries WHERE conversation_id = '-5189538420' ORDER BY timestamp DESC;"
```

说明：查库是**降序**（DESC）；RecentMessagesStrategy 拿到结果后会再按时间**升序**排、取最后 N 条，所以发给 AI 的「Conversation (recent)」块是旧→新。

结论：**从现有数据库（telegram_bot.db messages 表）看，AI 提交的「Conversation (recent)」与数据库中的对话内容和顺序一致；真正喂给 AI 的 recent 来自 memory_entries，需在有 `./data/memory.db` 的环境里用上面第二条 SQL 再对一遍。**
