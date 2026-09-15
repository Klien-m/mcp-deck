//! 库入口：公开不依赖桌面运行时的配置核心，desktop 特性启用 Tauri 宿主。
//! 命令分发与 Engine 独立，使核心测试可以在无窗口环境运行。

pub mod adapters;
pub mod commands;
pub mod engine;
pub mod model;
pub mod storage;

#[cfg(feature = "desktop")]
mod desktop {
    use crate::{
        commands::{self, Request},
        engine::Engine,
        model::Result,
    };
    use serde_json::Value;
    use std::sync::Mutex;

    #[tauri::command]
    /// 持有进程内互斥锁完成一次命令；初始化失败时将原始错误返回界面。
    /// 状态保留 `Result<Engine>`，因此加载失败仍能启动窗口显示错误，而非创建空工作区。
    pub fn dispatch(
        state: tauri::State<'_, Mutex<Result<Engine>>>,
        request: Request,
    ) -> Result<Value> {
        let mut lock = state.lock().map_err(|_| "应用状态异常，请重新启动")?;
        let engine = lock.as_mut().map_err(|e| e.clone())?;
        commands::dispatch(engine, request)
    }
}

#[cfg(feature = "desktop")]
/// 选择真实或隔离目录，初始化工作区并注册唯一的桌面 IPC 入口。
pub fn run() {
    use std::{path::PathBuf, sync::Mutex};
    // Debug 默认指向项目内样例目录；显式 MCP_DECK_HOME 可覆盖此选择。
    // Release 不编入样例路径，未设置隔离环境变量时使用当前用户目录。
    #[cfg(debug_assertions)]
    if std::env::var_os("MCP_DECK_HOME").is_none() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join(".local-dev");
        std::env::set_var("MCP_DECK_HOME", root.join("fixture-home"));
        std::env::set_var("MCP_DECK_DATA_DIR", root.join("fixture-data"));
    }
    let home = std::env::var_os("MCP_DECK_HOME")
        .map(PathBuf::from)
        .or_else(dirs::home_dir)
        .expect("无法确定用户目录");
    let data = std::env::var_os("MCP_DECK_DATA_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            if std::env::var_os("MCP_DECK_HOME").is_some() {
                return home.join(".mcp-deck-data");
            }
            dirs::data_dir()
                .unwrap_or_else(|| home.join(".local/share"))
                .join("com.mcpdeck.desktop")
        });
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(Mutex::new(engine::Engine::open(home, data)))
        .invoke_handler(tauri::generate_handler![desktop::dispatch])
        .run(tauri::generate_context!())
        .expect("MCP Deck 无法启动");
}
