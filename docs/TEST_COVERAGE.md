# 测试覆盖率

本文档说明如何使用 **cargo-llvm-cov** 统计本项目的单元测试与集成测试覆盖率。

## 索引

- [安装](#安装)
- [基本用法](#基本用法)
- [常用命令](#常用命令)
- [仅统计指定 crate](#仅统计指定-crate)
- [与 CI 集成](#与-ci-集成)
- [说明与注意](#说明与注意)

## 安装

需先安装 Rust 工具链与 `cargo-llvm-cov`，并确保已安装 `llvm-tools-preview`（首次运行 `cargo llvm-cov` 时会自动提示安装）：

```bash
# 安装 cargo-llvm-cov（一次性）
cargo install cargo-llvm-cov

# 若未安装 llvm-tools-preview，首次运行时会执行：
# rustup component add llvm-tools-preview
```

## 基本用法

在项目根目录（workspace 根）执行：

```bash
# 运行所有测试并输出终端覆盖率摘要
cargo llvm-cov --workspace

# 生成 HTML 报告（默认输出到 target/llvm-cov/html/index.html）
cargo llvm-cov --workspace --html

# 生成 LCOV 文件（便于 CI 或其它工具消费）
cargo llvm-cov --workspace --lcov --output-path lcov.info
```

- **与外部交互**：`cargo llvm-cov` 会编译并运行 `cargo test`，并基于 LLVM 插桩收集覆盖率，不依赖外部服务。
- **输出位置**：HTML 报告在 `target/llvm-cov/html/`；若指定 `--output-path`，LCOV 写入该路径。`target/` 已在 `.gitignore` 中，生成的 `lcov.info` 若在根目录也已在 `.gitignore` 中。

## 常用命令

| 命令 | 说明 |
|------|------|
| `cargo llvm-cov --workspace` | 全 workspace 测试 + 终端覆盖率 |
| `cargo llvm-cov --workspace --html` | 全 workspace 测试 + 生成 HTML 报告 |
| `cargo llvm-cov --workspace --lcov --output-path lcov.info` | 全 workspace 测试 + 生成 LCOV |
| `cargo llvm-cov --workspace --open` | 生成 HTML 并自动打开浏览器 |
| `cargo llvm-cov --workspace --no-fail-fast` | 测试失败也继续跑完并出覆盖率 |
| `cargo llvm-cov --workspace --ignore-run-fail` | 即使测试失败也输出覆盖率（慎用） |

## 仅统计指定 crate

只对某个成员 crate 做覆盖率（更快，适合开发时迭代）：

```bash
# 仅 memory-lance
cargo llvm-cov -p memory-lance

# 仅 telegram-bot（含其 tests/ 下的集成测试）
cargo llvm-cov -p telegram-bot

# 生成该 crate 的 HTML 报告
cargo llvm-cov -p memory-lance --html --open
```

## 与 CI 集成

在 CI 中通常只需终端摘要或 LCOV，用于门禁或上传到 Codecov/Coveralls 等：

```bash
# 仅输出覆盖率，不生成文件
cargo llvm-cov --workspace

# 生成 LCOV 供 CI 上传
cargo llvm-cov --workspace --lcov --output-path lcov.info
```

若需在 CI 中安装 `cargo-llvm-cov`，可使用：

```bash
cargo install cargo-llvm-cov
```

或使用 `taiki-e/install-action` 等 action 安装指定版本。

## 说明与注意

- **工具**：使用 [cargo-llvm-cov](https://github.com/taiki-e/cargo-llvm-cov)，基于 LLVM 源码级插桩，支持行/区域/分支覆盖率。
- **范围**：`--workspace` 会包含所有 workspace 成员的单元测试及各 crate 的 `tests/` 集成测试。
- **编译时间**：首次或 clean 后运行会带插桩重新编译，耗时会比普通 `cargo test` 长；后续增量会快很多。
- **排除**：可通过 `--exclude` 排除不需要统计的 crate；默认会排除仅作为依赖的第三方 crate。
- **与文档测试**：默认会运行 doc tests；若不需要可用 `--no-doc-tests`。

本项目的单元测试与集成测试规范见 [AGENTS.md](../AGENTS.md)；Telegram Bot 相关测试方案见 [TELEGRAM_BOT_TEST_PLAN.md](./TELEGRAM_BOT_TEST_PLAN.md)。
