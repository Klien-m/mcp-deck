//! 桌面可执行文件入口；目录选择和运行时初始化集中在库的 run 中。

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
fn main() {
    mcp_deck::run();
}
