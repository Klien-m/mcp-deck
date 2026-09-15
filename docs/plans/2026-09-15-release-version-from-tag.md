# Release Version from Tag Implementation Plan

**Goal:** 根据发布标签同步应用内部版本及七个安装包文件名中的版本号。

**Architecture:** 工作流在构建前执行 Node.js 脚本，严格解析数字版本标签并将两段版本补为三段。脚本仅替换 npm、Cargo、Tauri 配置及锁文件中的应用版本，现有 Tauri 打包与定制 DMG 脚本继续读取同一配置。

**Tech Stack:** Node.js 24、GitHub Actions、Tauri 2。

## 步骤与验收

1. 新增 `scripts/set-release-version.mjs`，通过环境变量读取标签，支持可选 v 前缀及两段 / 三段数字版本；无效标签在写文件前失败。
2. 在 Release 工作流中于安装 Rust、执行 npm ci、构建之前调用脚本，Cargo 锁文件同步后仍使用 `--locked`。
3. 添加 Node.js 原生测试：版本规范化、依赖与格式保持、重复执行、无效标签不写文件、缺失字段不写文件和 Windows CRLF。
4. 在临时目录使用真实配置验证更新与 Cargo 锁文件兼容性；执行 actionlint 和前端构建。
5. 更新 README，说明标签是发布版本来源，以及已存在标签的重新运行仍采用该标签内的旧工作流。

## 范围

不修改已发布标签和 Release，不改变本地日常开发的版本设置，不调整现有 DMG 布局或打包参数。
