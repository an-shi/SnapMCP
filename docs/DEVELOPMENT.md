# 开发、测试与打包

## 1. 环境

- Node.js ≥ 20.19（Vite 7 要求）
- Rust stable（>= 1.77，建议最新 stable）
- Windows：MSVC Build Tools + Windows SDK；WebView2 Runtime
- macOS：Xcode Command Line Tools
- Linux：`webkit2gtk`、`libxdo` 等 Tauri 依赖；截图仅 X11

安装依赖：

```bash
npm install
```

## 2. 开发

```bash
# 同时启动 Vite 与 Tauri
npm run tauri dev

# 只启动前端（mock 后端）
npm run dev
```

浏览器 mock 页面：

- 主界面：<http://localhost:1420/>
- 截图覆盖层：<http://localhost:1420/overlay.html>
- 人工确认：<http://localhost:1420/overlay.html?mode=interactive>

mock 模式会生成一张管理后台风格的假截图，所有文件/剪贴板操作为模拟，不会访问系统。

## 3. 检查与测试

```bash
npm run check         # vue-tsc 类型检查
npm run build         # 前端生产构建
cd src-tauri
cargo test --lib      # 标注渲染单测
cargo check
```

MCP 冒烟测试（需要桌面应用正在运行）：

```bash
npm run smoke:mcp
node scripts/smoke-mcp.mjs --capture
```

它会测试：

- legacy `initialize`
- `tools/list`
- `list_displays`
- 可选 `take_screenshot`
- modern `server/discover`

### 手动验收清单

- [ ] 冷启动无窗口，托盘出现图标，任务管理器内存约 20–25 MB
- [ ] `Ctrl/Cmd+Shift+X` 打开覆盖层（主屏）
- [ ] 拖动框选，尺寸标签正确，Shift 可锁定正方形
- [ ] 矩形/椭圆/箭头/直线/画笔/文字/高亮/马赛克/序号全部可绘制
- [ ] 文字支持中文、换行、底色
- [ ] `Ctrl+Z` / `Ctrl+Shift+Z` 撤销重做
- [ ] `Enter` 复制到剪贴板，粘贴图片验证
- [ ] `Ctrl+S` 保存 PNG；`Ctrl+Shift+S` 弹出另存为
- [ ] `Esc` 取消后覆盖层关闭，临时文件删除，busy 状态释放
- [ ] 关闭设置窗口后进程仍在托盘，MCP 仍可用
- [ ] 托盘菜单：截图、打开设置、打开文件夹、退出
- [ ] MCP `take_screenshot`（无标注）返回 PNG + 路径
- [ ] MCP `annotate_screenshot` 后图片可见箭头/文字/马赛克
- [ ] `capture_interactive` 能等待用户确认并返回结构化标注
- [ ] stdio 桥在应用已启动 / 未启动两种情况下都能初始化
- [ ] 修改快捷键后旧快捷键失效、新快捷键生效
- [ ] 开启/关闭开机自启
- [ ] 修改保存目录后新截图保存到新目录

## 4. 打包

```bash
# 完整安装包（NSIS/MSI 或 dmg/appimage/deb）
npm run tauri build

# 只构建可执行文件，跳过安装包
npm run tauri build -- --no-bundle
```

产物：

- Windows：`src-tauri/target/release/SnapMCP.exe`
- 安装包：`src-tauri/target/release/bundle/`

`Cargo.toml` 的 release profile 已设置：

```toml
codegen-units = 1
lto = true
opt-level = "s"
panic = "abort"
strip = true
```

实测参考（本仓库开发机，Windows 11 x64，2026-11）：

| 项目 | 数值 |
| --- | --- |
| Release 可执行文件 | **4.66 MB**（Windows x64，`--no-bundle`） |
| 无窗口托盘态工作集 | **~16 MB**（release）/ ~22 MB（debug） |
| stdio 桥进程 | 额外 ~11 MB |
| 打开设置 / 覆盖层 | WebView2 运行时开销，整体会升高 |
| 前端构建产物 | JS+CSS 约 180 KB（gzip < 60 KB） |

> 如需要进一步压缩体积，可移除未使用的 Tauri 插件、关闭 `image` 的 JPEG 支持，或把图标/字体做成更小的子集；当前优先保证功能完整。

## 5. 目录与状态文件

```text
%APPDATA%\SnapMCP\
├─ settings.json      # 快捷键、保存目录、MCP 端口、token
├─ runtime.json       # 实际端口、PID、token（stdio 桥使用）
└─ snapmcp.log        # 启动/工具调用/错误日志，自轮转（>2MB 重建）

%TEMP%\snapmcp\captures\
└─ cap_*.png          # 覆盖层临时截图，关闭即删除
```

默认截图目录：`图片\SnapMCP\`（可在设置中更改）。

## 6. 代码导航

| 需求 | 位置 |
| --- | --- |
| 增加一种前端标注工具 | `src/lib/types.ts` → `renderer.ts` → `OverlayView.vue` → `Toolbar.vue`；同步 `annotate.rs` 与 MCP schema |
| 增加一个 MCP 工具 | `src-tauri/src/mcp/protocol.rs` 的 `tools()` + `src-tauri/src/mcp/tools.rs` 的 `execute()` |
| 调整截图屏选择 | `src-tauri/src/capture.rs` 的 `DisplaySpec` / `capture_for_mcp` |
| 修改覆盖层窗口行为 | `src-tauri/src/overlay.rs` |
| 修改设置页 | `src/App.vue` + `src-tauri/src/commands.rs` |
| 修改 MCP 协议分支 | `src-tauri/src/mcp/server.rs` |

### 新增标注类型注意事项

1. 在 TypeScript `Annotation` 联合类型中加入判别式结构。
2. 在 `renderer.ts` 的 `drawAnnotation` 增加 Canvas 实现。
3. 在 `OverlayView.vue` 增加工具按钮/快捷键/拖拽构造逻辑（如需要）。
4. 在 `annotate.rs` 的 `Annotation` enum 增加 serde 变体；实现 Rust 渲染，注意 alpha 混合与越界裁剪。
5. 在 `mcp/protocol.rs` 的 `annotation_schema()` 中增加 JSON Schema。
6. 为 Rust 渲染添加单测（参考 `annotate.rs` 底部）。

## 7. 调试技巧

- 日志：设置页“打开配置目录”→ `snapmcp.log`；或直接：
  ```powershell
  Get-Content "$env:APPDATA\SnapMCP\snapmcp.log" -Tail 50
  ```
- MCP 端口/状态：
  ```powershell
  Invoke-RestMethod http://127.0.0.1:37621/health
  ```
- stdio 模式直接测试：
  ```powershell
  '{"jsonrpc":"2.0","id":1,"method":"tools/list"}' | .\SnapMCP.exe --mcp-stdio
  ```
- 覆盖层不出现：查看日志是否有“Overlay ...”与快捷键注册错误；Windows 下检查是否被其他快捷键占用。
- 不能粘贴图片：确认前端未出现“Tainted canvas”提示；覆盖层图片必须走 data URL。
- macOS 截图黑屏：给 SnapMCP 授予“屏幕录制”权限并重启应用。

## 8. 已知开发坑

1. **不要在覆盖层中使用 asset protocol 图片再导出 Canvas**：跨源会 taint，`toDataURL` 失败。
2. **覆盖层窗口必须在 `overlay_ready` 之前保持隐藏**，否则会先闪白/闪黑。
3. **`--mcp-stdio` 必须在 Tauri 初始化前处理**，否则单实例插件会拦截桥进程。
4. **GUI 子系统的 Windows exe 仍可用管道 stdio**，但不要向 stdout 打日志；所有诊断走 stderr/日志文件。
5. **图片/标注坐标要区分屏幕绝对坐标与图片相对坐标**，不要把 `region` 直接用于 `annotations`。
6. **修改协议版本支持时**同步更新 `SUPPORTED_VERSIONS`、`server/discover` 与文档；现代协议要求 `resultType`。
7. **Tauri 窗口 label 必须出现在 `capabilities/default.json` 的 windows 列表中**，否则新窗口的 IPC 权限可能不生效。
8. **覆盖层临时文件仅在 `%TEMP%\snapmcp` 下删除**，避免误删用户文件。

## 9. 图标

`src-tauri/icons/` 已包含生成的图标。修改后重新生成：

```bash
npx tauri icon src-tauri/icons/source-icon.png -o src-tauri/icons
```

## 10. 问题反馈时请附带

- 操作系统版本 + 显示器缩放比例
- `snapmcp.log` 最后 100 行
- `runtime.json`（如需隐藏 token，请先删除 token 字段）
- `cargo test --lib` 与 `npm run check` 结果
- MCP 客户端名称 + 接入方式（stdio / HTTP）+ 复现步骤
