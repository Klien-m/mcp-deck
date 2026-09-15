# DMG 安装界面定制

## 设计

用户选择极简浅灰、接近 macOS 原生风格。使用 660 × 420 点浅灰背景、居中安装标题与中文拖拽指引、左右原生图标和紧凑双折线箭头（»）、底部安装后启动提示。保留现有应用图标。

背景通过 macOS AppKit 脚本生成 2x PNG，记录逻辑尺寸，避免 Retina 屏幕模糊。窗口预留标题栏，图标坐标与箭头对齐。PNG 随仓库分发，常规构建无需生成背景。

## 打包

Tauri 负责构建与签名 app；固定版本的 dmgbuild 读取 `bundle.macOS.dmg` 并生成安装盘。本地与 CI 使用同一个脚本，支持 Apple Silicon 和 Intel 目标。

实机验收发现 Tauri 的 Finder 脚本虽然成功退出，最终保存的视图仍为默认白底、小图标及按大小排序。因此使用 dmgbuild 直接写入 `.DS_Store`，避免依赖 Finder 的异步保存；`grid_spacing` 显式设为 64，避免 Finder 拒绝过大的间距。

## 验收

1. 检查背景像素尺寸、144 DPI、脚本重复生成一致性。
2. 构建 app；模拟 CI 环境生成 DMG。
3. 挂载并检查真实 Finder 窗口、背景、图标、Applications 链接和应用签名。
4. 核查发布矩阵、版本与架构命名、改动范围。不修改业务代码。

## 本地验证结果（2026-09-15）

环境：macOS 14.8.4 / Apple Silicon。

- 前端 TypeScript / Vite 与 Rust release 构建通过。
- AppKit 生成器通过 `-Wall -Wextra -Werror` 编译；重复生成的 PNG 字节一致，尺寸 1320 × 840、144 DPI。
- `CI=true` 下，本地默认路径和 `--target aarch64-apple-darwin` 路径均成功生成 DMG。
- 挂载最终安装盘：校验和通过，`.DS_Store` 中背景类型为图片、图标为 128 点、无自动排序、坐标与配置一致；背景字节与仓库一致，Applications 指向 `/Applications`。
- 原生 Finder 截图确认标题、说明、背景、箭头、图标和底部提示完整显示。
- 安装盘内 app 的 `codesign --verify --deep --strict` 通过。打包不设置 `hide_extensions`，避免给已签名 app 增加 FinderInfo。
- 发布 YAML、macOS 双架构矩阵和 `git diff --check` 通过。
- 尚未在 GitHub Actions 和 Intel Mac 上执行；主仓库保持干净，修改保留在功能 worktree 中。
