//! 单元测试目录
//!
//! 与代码文件分离，集中放置 middleware 相关单元测试。
//! 与各 middleware 的交互：通过公开与 pub(crate) 接口进行测试。

mod memory_middleware_test;
mod persistence_middleware_test;
