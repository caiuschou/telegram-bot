# HandlerChain 抽离

## 概述

将 `HandlerChain` 从 bot-runtime 抽离为独立 crate `crates/handler-chain/`，便于复用与独立测试。

## 变更摘要

- **新增**：crates/handler-chain/（Cargo.toml、src/lib.rs、README.md）。
- **workspace**：根 Cargo.toml 增加成员 handler-chain。
- **bot-runtime**：依赖 handler-chain，删除 src/handler_chain.rs，保留 `pub use handler_chain::HandlerChain`。
- **验证**：cargo check / cargo test -p handler-chain、-p bot-runtime 通过。

动机、依赖、验收与后续建议见 [handler-chain-extraction-summary.md](handler-chain-extraction-summary.md)。
