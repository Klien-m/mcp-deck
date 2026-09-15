# Tag Release Implementation Plan

**Goal:** 基于 master 发布 v0.1，后续每次推送标签自动生成包含更新说明及 Windows、macOS、Linux 安装包的 GitHub Release。

**Architecture:** 四个矩阵任务构建 Windows x64、macOS arm64 / x64、Linux x64，临时产物通过 Actions artifacts 传给发布任务。全部成功后，发布任务检查安装包数量，从标签之间的 Git 历史生成中文更新说明，上传七个安装包并公开 Release。

**Tech Stack:** GitHub Actions、Node.js 24、Rust stable、Tauri 2、GitHub CLI。

## 实现步骤

1. 从 origin/master 创建 `req/kouxinxin/2026/09/tag-release` 分支。
2. 新增 `.github/workflows/release.yml`：所有标签推送触发，支持在指定标签手动重试，安装 Linux 依赖并缓存 npm / Cargo。
3. 新增 `scripts/release-notes.mjs`：首次发布列出完整历史，后续列出最近祖先标签以来的提交及比较链接。
4. 更新 README 的下载入口、产物说明和标签发布流程。保留现有本地打包设置，在 CI 命令中指定平台产物。
5. 使用 actionlint、现有测试和本地 macOS 打包验证；在临时仓库验证首次发布和增量更新说明。
6. 将发布改动快进到 master，创建 annotated tag `v0.1`，推送 master 和标签，检查 GitHub Actions 与 Release 产物。

## 发布约束

- 不包含尚未合入 master 的模块重构提交。
- 仅发布任务拥有 `contents: write`，构建失败不发布残缺版本。
- 标签指向包含工作流的提交；重试沿用已有 Release 并替换同名资产。
- 应用版本由已提交的 package.json、Cargo.toml 和 tauri.conf.json 决定；v0.1 对应 0.1.0。
- 当前没有分发证书，macOS 使用 ad hoc 签名，Windows 不签名；更新说明明确标识。
