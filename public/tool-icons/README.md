# 工具图标来源

更新日期：2026-09-14。图标随应用离线打包，用于识别对应的第三方工具。保留原始颜色和比例，不添加主题滤镜。优先采用官网 SVG；透明画布留白通过显示视口校准，原始图案不重绘。品牌和商标归各自所有者所有；展示图标不表示获得其背书。

| 工具 | 原始资源 |
| --- | --- |
| Codex | [OpenAI 官方品牌包](https://cdn.openai.com/brand/openai-logos.zip)中的 `OpenAI-logos/SVGs/OAI_OpenAI-Blossom_Black.svg`，与 [Codex 官网](https://openai.com/codex/)当前 PNG 的花结主体对应 |
| Claude Code | 与 Claude Desktop 共用从 [Claude 官网](https://claude.com/)字标中导出的星形 SVG，保留原始路径与橙色 |
| Cursor | [官方品牌包](https://cursor.com/en-US/brand)中的 `General Logos/Cube/SVG/CUBE_2D_LIGHT.svg` |
| Gemini CLI | [官方品牌页](https://geminicli.com/brand-kit/)提供的[彩色终端 SVG](https://geminicli.com/brandassets/gemini-cli-icon_full-color.svg) |
| OpenCode | [官方品牌页](https://opencode.ai/brand)提供的 `opencode-logo-light-square.svg` |
| GitHub Copilot CLI | [GitHub Octicons 的 Copilot 标识](https://github.com/github/octicons/blob/main/icons/copilot-24.svg)，MIT 许可全文见 `LICENSE.octicons` |
| VS Code | [官方品牌包](https://code.visualstudio.com/brand)中的蓝色 `vscode.svg` |
| Windsurf | [官方网站图标](https://windsurf.com/favicon.svg)，[品牌说明](https://windsurf.com/brand) |
| Kiro | [官方网站图标](https://kiro.dev/icon.svg?fe599162bb293ea0) |
| Cline | [官方品牌页](https://cline.bot/brand)提供的 [机器人 SVG](https://cline.bot/assets/branding/brand/General%20Logos/Bot/SVG/BOT_LIGHT.svg) |
| Roo Code | [官方扩展仓库的袋鼠 SVG](https://github.com/RooCodeInc/Roo-Code/blob/main/src/assets/icons/icon.svg) |
| Claude Desktop | 与 Claude Code 共用 `claude.svg` |

Codex、Cursor、VS Code、Cline、Roo Code、Gemini CLI SVG 与官方文件逐字节一致。Claude SVG 单独封装官网字标中 `fill="#D97757"` 的星形路径，未重绘路径或修改填色。OpenCode 的 SVG 只从官方品牌页资源中的 data URI 解码导出，原始路径、颜色及视口保持一致。Codex 使用官方黑色花结 SVG，不包含 PNG 版本的浅色底座及阴影。

显示时保留 SVG 的原始比例；Codex 官方 SVG 的透明边距通过显示视口校准，原始文件保持不变。主列表为 26 px，选择菜单为 24 px，服务列表为 16 px。`sources.json` 记录资源来源、版本、文件 SHA-256、使用该资源的工具，以及需校准图标的 `displayBounds`（左、上、右、下）。
