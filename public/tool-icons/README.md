# 工具图标来源

更新日期：2026-09-14。图标随应用离线打包，用于识别对应的第三方工具。保留原始颜色和比例，不添加主题滤镜。优先采用官网 SVG；PNG 的透明画布留白通过显示视口校准，原始图案不重绘。品牌和商标归各自所有者所有；展示图标不表示获得其背书。

| 工具 | 原始资源 |
| --- | --- |
| Codex | [Codex 官网](https://openai.com/zh-Hans-CN/codex/)当前展示的 [ChatGPT / OpenAI Blossom PNG](https://images.ctfassets.net/kftzwdyauwt9/77tJ5U1tgxHMZflZ5m4Z24/ace4d8b6ad200d87ebcb69c466344343/Blossom_4k_Icon_1.png?w=256) |
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

Cursor、VS Code、Cline、Roo Code、Gemini CLI SVG 与官方文件逐字节一致。Claude SVG 单独封装官网字标中 `fill="#D97757"` 的星形路径，未重绘路径或修改填色。OpenCode 的 SVG 只从官方品牌页资源中的 data URI 解码导出，原始路径、颜色及视口保持一致。Codex 保留已确认的官网 PNG 图标。

显示时保留 SVG 的原始比例；Codex PNG 的透明边距通过 SVG 显示视口校准。主列表为 26 px，选择菜单为 24 px，服务列表为 16 px。`sources.json` 记录资源来源、版本、文件 SHA-256、使用该资源的工具，以及需校准图标的 `displayBounds`（左、上、右、下）。
