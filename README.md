# SnapMCP

> 一款轻量的桌面截图与标注工具，原生内置 **Model Context Protocol (MCP)**。
> 框选屏幕、画箭头、写序号、一键复制；也可以让 Claude / Cursor / 其他 AI 客户端直接“看见”并标注你的屏幕。

SnapMCP 使用 **Tauri 2 + Rust + Vue 3 + TypeScript** 构建。没有窗口时常驻托盘，闲置内存约 20–25 MB；截图覆盖层只在需要时创建，关闭后销毁。

---

## 功能

- **全局快捷键截图**：默认 `Ctrl/Cmd + Shift + X`，可在设置中修改。
- **全屏框选覆盖层**：冻结屏幕、半透明遮罩、尺寸提示、Shift 约束正方形。
- **矢量标注**：矩形、椭圆、箭头、直线、画笔、文字、高亮、马赛克、带序号步骤徽章。
- **完整编辑体验**：撤销 / 重做、颜色与粗细切换、文字底色、重新框选。
- **一键输出**：`Enter` 复制到剪贴板，`Ctrl+S` 快速保存 PNG，支持另存为。
- **内置 MCP Server**：HTTP 与 stdio 两种接入方式，支持截图、标注、人工确认截图和显示器枚举。
- **托盘常驻 + 开机自启**：关闭设置窗口后继续驻留系统托盘。
- **极小体积、极低闲置开销**：Rust 后端、无窗口启动、按需创建 WebView。

## 技术栈

| 层 | 技术 |
| --- | --- |
| 桌面壳 / 托盘 / 全局快捷键 | Tauri 2 + Rust |
| 屏幕捕获 | [`xcap`](https://crates.io/crates/xcap)（Windows GDI / macOS CoreGraphics / Linux X11） |
| 前端 | Vue 3 + TypeScript + Vite |
| 标注画布 | HTML5 Canvas + 自研渲染器 |
| MCP | 自研轻量 Server：Axum（HTTP）+ 原生 stdio 桥接 |
| MCP 协议 | 双代兼容：`2026-07-28`（现代无状态）与 `2025-11-25` 及更早（`initialize` 握手） |
| 图片处理 | `image` / `imageproc` / `ab_glyph`（Rust 侧字体渲染） |

---

## 快速开始

### 环境要求

- Windows 10/11（推荐）或 macOS 11+ / 主流 Linux 桌面
- Rust stable ≥ 1.77 与 MSVC Build Tools（Windows）
- Node.js ≥ 20.19
- WebView2 Runtime（Windows 10/11 通常已内置）
- macOS 首次使用需在“系统设置 → 隐私与安全性 → 屏幕录制”中授权
- Linux 需要 X11；当前 xcap 对 Wayland 截图支持不完整

### 开发

```bash
npm install
npm run tauri dev
```

### 构建

```bash
npm run tauri build
```

产物位于 `src-tauri/target/release/bundle/`。只想构建单文件可执行程序：

```bash
npm run tauri build -- --no-bundle
# src-tauri/target/release/SnapMCP.exe
```

### 浏览器预览前端

不需要 Rust 也能检查界面：

```bash
npm run dev
# 主界面    http://localhost:1420/
# 截图覆盖层 http://localhost:1420/overlay.html
# 人工确认模式 http://localhost:1420/overlay.html?mode=interactive
```

浏览器预览使用内置 mock 截图与 mock 后端，不读写系统剪贴板或文件。

---

## 使用

1. 按 `Ctrl/Cmd + Shift + X`，或在托盘中点击“截取屏幕”。
2. 拖动框选区域；`Esc` 取消。
3. 选好区域后自动进入标注模式，底部出现工具栏。
4. 标注完成后：
   - `Enter` / 点击“复制” → 粘贴到聊天窗口、文档或 AI 对话
   - `Ctrl+S` → 保存到截图目录
   - `Ctrl+Shift+S` → 另存为
   - `Esc` → 放弃本次截图

### 快捷键

| 操作 | 快捷键 |
| --- | --- |
| 矩形 / 椭圆 | `R` / `E` |
| 箭头 / 直线 | `A` / `L` |
| 画笔 / 文字 | `P` / `T` |
| 高亮 / 马赛克 | `H` / `M` |
| 序号徽章 | `S` |
| 撤销 / 重做 | `Ctrl+Z` / `Ctrl+Shift+Z` |
| 复制 | `Enter` 或 `Ctrl+C` |
| 保存 / 另存为 | `Ctrl+S` / `Ctrl+Shift+S` |
| 发送给 AI（人工确认模式） | `Ctrl+Enter` |

> macOS 上将 `Ctrl` 换为 `Cmd` 即可。

---

## 接入 MCP

### 方式一：Claude Desktop（stdio，推荐）

编辑 `claude_desktop_config.json`（Windows 路径通常为 `%APPDATA%\Claude\claude_desktop_config.json`）：

```json
{
  "mcpServers": {
    "snapmcp": {
      "command": "C:\\Program Files\\SnapMCP\\SnapMCP.exe",
      "args": ["--mcp-stdio"]
    }
  }
}
```

重启 Claude Desktop。SnapMCP 会自动作为单实例托盘应用启动；如果它已经在运行，stdio 桥会直接复用现有实例。

### 方式二：HTTP（Streamable HTTP）

设置界面会显示实际地址，默认：

```text
http://127.0.0.1:37621/mcp
```

适用于支持 Streamable HTTP 的 MCP 客户端，例如：

```bash
claude mcp add --transport http snapmcp http://127.0.0.1:37621/mcp
```

服务仅绑定 loopback，并校验 `Origin`；不会暴露到局域网。

### 工具

| 工具 | 作用 |
| --- | --- |
| `take_screenshot` | 截取主屏、指定屏、全部屏幕或指定区域，可同时应用标注 |
| `annotate_screenshot` | 对最近一次截图或指定图片追加标注 |
| `capture_interactive` | 打开覆盖层，让用户亲自框选/标注并确认，然后返回给 AI |
| `list_displays` | 列出显示器几何信息、缩放、主屏/光标所在屏 |

示例对话：

> 截取当前屏幕，用红色箭头指向用户列表里的“删除”按钮，并在旁边写上“① 删除用户”。

> 帮我截取主显示器，把邮箱地址打码，然后把 PNG 路径和图片发给我。

> 请让用户自己框选报错弹窗并加上标注，然后把图片发给我分析。

完整的工具参数、坐标约定、标注 Schema 与客户端配置见 **[docs/MCP.md](docs/MCP.md)**。

---

## 标注数据

前后端及 MCP 共用同一套标注结构：

```json
[
  {
    "type": "arrow",
    "from": [620, 480],
    "to": [900, 300],
    "color": "#FF453A",
    "strokeWidth": 4
  },
  {
    "type": "text",
    "x": 120,
    "y": 260,
    "text": "① 点击删除",
    "fontSize": 32,
    "color": "#FFFFFF",
    "bgColor": "#0B1220CC"
  },
  {
    "type": "step",
    "x": 930,
    "y": 320,
    "index": 1,
    "color": "#FF453A"
  }
]
```

支持的 `type`：`rect`、`ellipse`、`arrow`、`line`、`pen`、`text`、`highlight`、`mosaic`、`step`。
坐标与尺寸都以 **截图像素** 为单位，原点为图片左上角。

---

## 性能与体积

在测试机（Windows 11，1920×1080）上：

- **Release 可执行文件**：**约 4.7 MB**（未包含安装器）
- **无窗口托盘态**：release 版约 **16 MB**；debug 版约 22 MB
- **stdio 桥进程**：额外约 11 MB；即使同时常驻也通常低于 30 MB
- **设置窗口打开时**：受 WebView2 运行时影响，整体工作集会升高（约 50–100 MB，取决于 WebView2 复用情况）
- **覆盖层**：仅在截图期间创建，关闭后销毁；图片以同源 data URL 加载，避免 Canvas 跨域污染
- **冷启动**：不创建任何窗口，托盘进程启动很快；首次唤起覆盖层需要初始化 WebView2，通常 1–2 秒

Release 构建启用 `lto = true`、`opt-level = "s"`、`codegen-units = 1`、`strip = true`、`panic = "abort"`。

> 包体大小以本机实际构建结果为准，详见 [docs/DEVELOPMENT.md](docs/DEVELOPMENT.md)。

## 项目结构

```text
SnapMCP/
├─ src/                        # Vue 3 前端
│  ├─ views/OverlayView.vue    # 框选 + 标注覆盖层（核心 UI）
│  ├─ components/Toolbar.vue   # 浮动工具栏
│  ├─ lib/renderer.ts          # Canvas 标注渲染 / 导出合成
│  ├─ lib/bridge.ts            # Tauri invoke 封装 + 浏览器 mock
│  ├─ lib/mock.ts              # 浏览器预览用的假后台截图
│  └─ App.vue                  # 设置 / 状态主页
├─ src-tauri/
│  ├─ src/capture.rs           # xcap 捕获、多屏合成、区域裁剪
│  ├─ src/annotate.rs          # Rust 侧矢量标注渲染（MCP 自动标注）
│  ├─ src/overlay.rs           # 覆盖层窗口、生命周期、临时文件
│  ├─ src/commands.rs          # 前端命令：保存/复制/设置/引导
│  ├─ src/mcp/                 # MCP Server（HTTP + 协议 + 工具）
│  ├─ src/cli.rs               # --mcp-stdio 桥接
│  ├─ src/tray.rs              # 系统托盘
│  └─ src/lib.rs               # 应用入口、插件、单实例
├─ docs/
│  ├─ ARCHITECTURE.md
│  ├─ MCP.md
│  └─ DEVELOPMENT.md
└─ scripts/smoke-mcp.mjs       # 本地 MCP 冒烟测试
```

## 安全模型

- HTTP MCP 仅监听 `127.0.0.1`，并拒绝非本地 `Origin`。
- 可选 Bearer Token（`%APPDATA%\SnapMCP\settings.json` 的 `authToken`）；stdio 桥会自动从 `runtime.json` 读取。
- 截图临时文件位于系统临时目录，覆盖层关闭即删除。
- 不启用 Tauri asset protocol；覆盖层图片通过 data URL 同源加载。
- 截图与标注始终由用户主动触发，或由人工确认的 `capture_interactive` 完成。

## 已知限制

- 交互式覆盖层目前只覆盖 **主显示器**；多屏自动截图请使用 MCP 的 `display` / `region`。
- 混合 DPI 多显示器环境下，覆盖层以主屏缩放比例为基准。
- 暂不支持 Wayland 原生截图（xcap 限制）。
- 标注是追加式的；暂不支持选中已有标注后再移动/缩放（可撤销或重新框选）。
- MCP HTTP 端不支持已废弃的 SSE GET 流；所有响应以 JSON 返回。
- 未实现 HTTP 的 CORS，仅供桌面/CLI 客户端使用。

## 开发文档

- [架构与设计](docs/ARCHITECTURE.md)
- [MCP 接口文档](docs/MCP.md)
- [开发、测试与打包](docs/DEVELOPMENT.md)

## License

MIT
