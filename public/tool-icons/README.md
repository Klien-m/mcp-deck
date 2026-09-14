# 工具图标来源

更新日期：2026-09-14。图标随应用离线打包，用于识别对应的第三方工具。保留原始颜色和比例，不添加主题滤镜。应用包资源的 Dock 透明边距由显示视口统一校准，原始文件不重绘。品牌和商标归各自所有者所有；展示图标不表示获得其背书。

| 工具 | 原始资源 |
| --- | --- |
| Codex | 本机 ChatGPT 26.903.71938 应用包中的 `icon-codex-light.png`，蓝色云朵与圆角浅色底座，外沿透明 |
| Claude Code | 与 Claude Desktop 共用 `claude-desktop.png` |
| Cursor | 本机 Cursor 3.16.2 应用包中的 `Cursor.icns` |
| Gemini CLI | [官方网站图标](https://geminicli.com/icon.png) |
| OpenCode | [官方网站图标](https://opencode.ai/favicon-96x96-v3.png) |
| GitHub Copilot CLI | [GitHub Octicons 的 Copilot 标识](https://github.com/github/octicons/blob/main/icons/copilot-24.svg)，MIT 许可全文见 `LICENSE.octicons` |
| VS Code | 本机 Visual Studio Code 1.135.0 应用包中的 `Code.icns` |
| Windsurf | [官方网站图标](https://windsurf.com/favicon.svg)，[品牌说明](https://windsurf.com/brand) |
| Kiro | [官方网站图标](https://kiro.dev/icon.svg?fe599162bb293ea0) |
| Cline | [官方网站图标](https://cline.bot/assets/branding/favicons/favicon-256x256.png) |
| Roo Code | [官方扩展仓库图标](https://github.com/RooCodeInc/Roo-Code/blob/main/src/assets/icons/icon.png) |
| Claude Desktop | 本机 Claude 1.44121.4 应用包中的 `electron.icns` |

Cursor、VS Code 与 Claude 的图标使用 `iconutil -c iconset` 提取原有的 128 × 128 PNG 表示；Codex 直接使用 1024 × 1024 的原始透明 PNG，避免 `app.icns` 中不透明白色方底的表示。文件均未重绘或缩放。

显示时，以原图主体的可见边界作为 SVG `viewBox`，使各图标主体填充相同尺寸，而非让包含不同留白的整张图片填充。主列表为 26 px，选择菜单为 24 px，服务列表为 16 px。`sources.json` 记录资源来源、版本、文件 SHA-256、使用该资源的工具，以及需校准图标的 `displayBounds`（左、上、右、下）。
