//! 适配器公开入口：注册表描述能力和默认位置，codec 转换条目，document 编辑文档。
//! 转换与补丁函数不执行文件写入；文件保护和事务由 storage / engine 负责。

mod codec;
mod document;
mod paths;
mod registry;

pub use codec::{decode, encode};
pub use document::{parse, patch};
pub use paths::{default_targets, legacy_default_path_notice, migrate_default_targets};
pub use registry::{get, registry, Adapter, Dialect};
