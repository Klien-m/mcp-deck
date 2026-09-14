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
pub fn run() {
    use std::{path::PathBuf, sync::Mutex};
    // Debug builds always use project-local fixtures unless an explicit test home is supplied.
    // Release builds use the user's real home and never inherit a compiled-in fixture path.
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
