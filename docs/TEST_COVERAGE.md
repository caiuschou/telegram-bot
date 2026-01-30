# 测试覆盖率

使用 **cargo-llvm-cov** 统计单元测试与集成测试覆盖率。

## 安装与基本用法

```bash
cargo install cargo-llvm-cov
# 首次可能提示：rustup component add llvm-tools-preview

cargo llvm-cov --workspace                    # 全 workspace，终端摘要
cargo llvm-cov --workspace --html             # 生成 HTML（target/llvm-cov/html/）
cargo llvm-cov --workspace --lcov --output-path lcov.info   # LCOV，供 CI
```

## 常用命令

| 命令 | 说明 |
|------|------|
| `cargo llvm-cov --workspace` | 全量测试 + 覆盖率 |
| `cargo llvm-cov -p telegram-bot` | 仅指定 crate |
| `cargo llvm-cov --workspace --open` | 生成 HTML 并打开 |

## 说明

- 工具：[cargo-llvm-cov](https://github.com/taiki-e/cargo-llvm-cov)；范围含各成员单元测试与 `tests/` 集成测试。
- 首次或 clean 后编译较慢；可 `--exclude` 排除 crate；默认不跑 doc test 可加 `--no-doc-tests`。

测试规范见 [AGENTS.md](../AGENTS.md)；Bot 测试方案见 [TELEGRAM_BOT_TEST_PLAN.md](TELEGRAM_BOT_TEST_PLAN.md)。
