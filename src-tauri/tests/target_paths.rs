//! 大小写和分隔符别名需要在 Windows 文件系统上验证；纯比较矩阵另有单元测试。
#![cfg(windows)]

use mcp_deck::engine::Engine;
use std::path::{Path, PathBuf};

fn alias(path: &Path) -> PathBuf {
    let path = path.to_string_lossy().replace('\\', "/").to_lowercase();
    PathBuf::from(path.strip_prefix("//?/").unwrap_or(&path))
}

fn component_aliases(path: &Path) -> Vec<PathBuf> {
    let path = alias(path).to_string_lossy().into_owned();
    vec![
        PathBuf::from(path.replacen('/', "/./", 1)),
        PathBuf::from(path.replace('/', "//")),
        PathBuf::from(path),
    ]
}

#[test]
fn target_and_export_guards_reject_windows_aliases_of_protected_files() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let mut engine = Engine::open(root.join("FixtureHome"), root.join("McpDeckData")).unwrap();
    let cursor = engine
        .workspace
        .targets
        .iter()
        .find(|target| target.id == "cursor")
        .unwrap()
        .clone();
    let original_revision = engine.workspace.revision;

    for target_alias in component_aliases(Path::new(&cursor.path)) {
        let mut duplicate = engine
            .workspace
            .targets
            .iter()
            .find(|target| target.id == "codex")
            .unwrap()
            .clone();
        duplicate.path = target_alias.to_string_lossy().into();
        assert!(engine
            .save_target(duplicate)
            .unwrap_err()
            .contains("另一个目标"));
        assert!(engine
            .save_export("cursor", &[], false, &target_alias)
            .unwrap_err()
            .contains("不能覆盖"));
    }
    for workspace_alias in component_aliases(&engine.data_dir.join("workspace.json")) {
        let mut protected = cursor.clone();
        protected.path = workspace_alias.to_string_lossy().into();
        assert!(engine
            .save_target(protected)
            .unwrap_err()
            .contains("数据目录"));
        assert!(engine
            .save_export("cursor", &[], false, &workspace_alias)
            .unwrap_err()
            .contains("不能覆盖"));
    }
    assert_eq!(engine.workspace.revision, original_revision);
    assert!(!Path::new(&cursor.path).exists());
}
