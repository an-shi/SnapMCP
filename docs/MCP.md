# SnapMCP MCP 接口文档

SnapMCP 在应用进程内运行一个轻量级 MCP Server。它同时支持：

1. **stdio**：`SnapMCP.exe --mcp-stdio`，适合 Claude Desktop 这类以子进程方式启动服务器的客户端。
2. **Streamable HTTP**：`http://127.0.0.1:<port>/mcp`，适合原生支持 HTTP 的 MCP 客户端。

服务器内部实际只有一个 HTTP 实现；stdio 是一个极薄的 JSON-RPC 双向转发桥。这样 UI、stdio、HTTP 共用同一份截图状态与工具实现。

---

## 1. 协议版本与兼容性

SnapMCP 是 **双代（dual-era）MCP Server**：

| 客户端类型 | 识别方式 | SnapMCP 行为 |
| --- | --- | --- |
| Legacy（`2025-11-25` 及更早） | 先发 `initialize` | 返回 `protocolVersion`、`capabilities`、`serverInfo`；不强制会话 |
| Modern（`2026-07-28`） | 请求 `params._meta` 中带 `io.modelcontextprotocol/protocolVersion` | 无状态处理；每个结果带 `resultType: "complete"` 和 `_meta` serverInfo |
| Modern 探测 | `server/discover` | 返回 `supportedVersions`、`capabilities`、`serverInfo`、instructions |

支持的版本列表：

```json
["2026-07-28", "2025-11-25", "2025-06-18", "2025-03-26", "2024-11-05"]
```

实现要点：

- 不签发 `Mcp-Session-Id`；legacy 客户端可以使用无会话模式。
- 不提供已废弃的 SSE GET 流；`GET /mcp` 返回 `405`，客户端应使用 POST + JSON 响应。
- 现代协议的可选 `MCP-Protocol-Version` / `Mcp-Method` / `Mcp-Name` 头会被校验（若提供且不匹配则返回 `400` / `-32020`），但不强制客户端发送，以兼容实现较宽松的客户端。
- JSON-RPC batching（数组）已在新版规范中移除，SnapMCP 返回 `400`。
- 现代 `tools/list` / `server/discover` 带 `ttlMs` 与 `cacheScope`。

---

## 2. 服务发现

### `GET /health`

无需认证（只监听 loopback）：

```json
{
  "ok": true,
  "name": "snapmcp",
  "version": "0.1.0",
  "port": 37621,
  "captureBusy": false,
  "mcp": {
    "running": true,
    "port": 37621,
    "endpoint": "http://127.0.0.1:37621/mcp",
    "tokenRequired": false,
    "error": null
  }
}
```

### `runtime.json`

应用启动并成功绑定端口后会写入：

- Windows：`%APPDATA%\SnapMCP\runtime.json`
- macOS：`~/Library/Application Support/SnapMCP/runtime.json`
- Linux：`~/.config/snapmcp/runtime.json`

```json
{
  "pid": 12345,
  "port": 37621,
  "endpoint": "http://127.0.0.1:37621/mcp",
  "token": "",
  "version": "0.1.0"
}
```

端口被占用时，SnapMCP 会从首选端口向后尝试最多 20 个端口，并写入实际端口。stdio 桥与设置界面都从这里读取真实地址。

---

## 3. 客户端配置

### 3.1 Claude Desktop（stdio）

`claude_desktop_config.json`：

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

macOS：

```json
{
  "mcpServers": {
    "snapmcp": {
      "command": "/Applications/SnapMCP.app/Contents/MacOS/SnapMCP",
      "args": ["--mcp-stdio"]
    }
  }
}
```

如果 SnapMCP 尚未运行，桥会自动启动 `SnapMCP.exe --hidden`，最多等待 10 秒；如果已经运行，则直接复用现有实例。

### 3.2 Claude Code（HTTP）

```bash
claude mcp add --transport http snapmcp http://127.0.0.1:37621/mcp
```

### 3.3 通用 HTTP 配置

```json
{
  "mcpServers": {
    "snapmcp": {
      "type": "http",
      "url": "http://127.0.0.1:37621/mcp"
    }
  }
}
```

部分客户端使用 `"transport": "http"` 或顶层 `"url"`，请按客户端文档调整；服务端地址不变。

---

## 4. 工具

### 4.1 `list_displays`

**参数**：无。

**返回**：文本 JSON：

```json
{
  "ok": true,
  "coordinateSpace": "absolute physical virtual-desktop pixels",
  "monitors": [
    {
      "index": 0,
      "name": "Unknown Monitor 65537",
      "x": 0,
      "y": 0,
      "width": 1920,
      "height": 1080,
      "scaleFactor": 1.0,
      "isPrimary": true,
      "isCursor": true
    }
  ],
  "cursorIndex": 0
}
```

`region` 使用与这里的 `x/y/width/height` 相同的绝对物理坐标空间。

---

### 4.2 `take_screenshot`

截取屏幕并可选应用标注，返回图片内容与 PNG 路径。

#### 参数

| 参数 | 类型 | 默认 | 说明 |
| --- | --- | --- | --- |
| `display` | string \| integer | `"primary"` | `"primary"` / `"cursor"` / `"all"` / 0-based 显示器索引；提供 `region` 时忽略 |
| `region` | object | — | 绝对虚拟桌面区域 `{x,y,width,height}`（物理像素） |
| `annotations` | array | `[]` | 标注数组，最多 200 个；坐标相对 **返回的图片** |
| `savePath` | string | 自动 | PNG 输出路径；相对路径位于截图目录 |
| `copyToClipboard` | boolean | `false` | 是否同时复制到剪贴板 |
| `includeImage` | boolean | `true` | 为 `false` 时只返回路径与元数据，节省上下文 |

#### 坐标语义

- `display`、`region`：绝对物理虚拟桌面坐标，与 `list_displays` 一致。
- `annotations`：相对于**最终截图图片**的左上角，单位是图片像素。
- 如果同时提供 `region`，最终图片是裁剪结果，标注坐标相对于裁剪后的图片。

> 推荐工作流：先 `take_screenshot`（不标注）→ 看图片与元数据 → 用 `annotate_screenshot` 按图片像素坐标精确标注。  
> 或者直接 `take_screenshot` 并给出近似标注，用于快速生成说明图。

#### 返回内容

1. `text`：`SnapMCP result:` 加一段 JSON，包含：
   - `path`、`width`、`height`
   - `region`、`display`
   - `annotationsRequested` / `annotationsApplied` / `annotationsSkipped`
   - `warnings`、`copiedToClipboard`
2. `image`（`includeImage` 为 true）：`image/png` 的 base64 数据，`mimeType: "image/png"`。

示例：

```json
{
  "name": "take_screenshot",
  "arguments": {
    "display": "primary",
    "annotations": [
      {
        "type": "arrow",
        "from": [640, 520],
        "to": [930, 310],
        "color": "#FF453A",
        "strokeWidth": 5
      },
      {
        "type": "text",
        "x": 120,
        "y": 300,
        "text": "① 点击删除用户",
        "fontSize": 30,
        "color": "#FFFFFF",
        "bgColor": "#0B1220CC"
      }
    ],
    "savePath": "user-list-annotated.png"
  }
}
```

---

### 4.3 `annotate_screenshot`

对最近一次截图或指定图片追加标注。

| 参数 | 类型 | 默认 | 说明 |
| --- | --- | --- | --- |
| `sourcePath` | string | 最近截图 | PNG/JPEG 源文件 |
| `annotations` | array | 必填 | 标注数组 |
| `savePath` | string | 自动 | 新 PNG 路径 |
| `copyToClipboard` | boolean | `false` | 是否复制结果 |
| `includeImage` | boolean | `true` | 是否返回图片 |

返回结构同 `take_screenshot`，额外包含 `sourcePath`。

---

### 4.4 `capture_interactive`

打开 SnapMCP 的覆盖层，让用户亲自框选、标注并确认。适合目标不明确、需要人工确认、或需要人机协作的场景。

| 参数 | 类型 | 默认 | 说明 |
| --- | --- | --- | --- |
| `hint` | string | — | 显示在覆盖层顶部的提示，最多 200 字符 |
| `timeoutSeconds` | integer | 180 | 5–600 秒 |
| `includeImage` | boolean | `true` | 是否返回图片 |

返回：

- 用户确认后：图片内容 + JSON（包含 `path`、`width`、`height`、`region`、`display`、结构化 `annotations`）。
- 用户取消或超时：工具结果 `isError: true`，AI 可据此继续询问用户。

人工确认模式下，用户可以画任何前端支持的标注；`Ctrl+Enter` 或“发送给 AI”完成。

---

## 5. 标注 Schema

坐标与尺寸单位都是**图片像素**。颜色支持：

- `#RGB` / `#RGBA` / `#RRGGBB` / `#RRGGBBAA`
- `rgb(r,g,b)` / `rgba(r,g,b,a)`
- `red`、`yellow`、`green`、`blue`、`white`、`black`、`gray`、`orange`、`purple`、`pink`、`cyan`、`magenta`
- `none` / `transparent`

### 5.1 矩形 `rect`

```json
{"type":"rect","x":100,"y":200,"w":300,"h":80,"color":"#FF453A","strokeWidth":4,"fill":"#FF453A22"}
```

- `fill` 可选，带 alpha 的颜色会正确混合。
- 别名：`rectangle`、`box`。

### 5.2 椭圆 `ellipse`

```json
{"type":"ellipse","x":100,"y":200,"w":300,"h":180,"color":"#32D74B","strokeWidth":4}
```

- `x,y,w,h` 是外接矩形。
- 别名：`circle`、`oval`。

### 5.3 箭头 `arrow`

```json
{"type":"arrow","from":[600,500],"to":[900,300],"color":"#FF453A","strokeWidth":5,"headSize":28}
```

- `from` / `to` 支持 `[x,y]` 或 `{"x":..,"y":..}`。
- `headSize` 可选，默认约 `strokeWidth * 5`。

### 5.4 直线 `line`

```json
{"type":"line","from":[100,100],"to":[800,600],"color":"#0A84FF","strokeWidth":4}
```

### 5.5 画笔 `pen`

```json
{"type":"pen","points":[[10,20],[30,40],[60,50]],"color":"#FFD60A","strokeWidth":4}
```

- 最多 5000 个点。

### 5.6 文字 `text`

```json
{
  "type":"text",
  "x":120,
  "y":300,
  "text":"① 点击删除\n② 确认操作",
  "fontSize":30,
  "color":"#FFFFFF",
  "bgColor":"#0B1220CC",
  "padding":10
}
```

- 支持中文与换行。
- 默认 `fontSize: 28`、`color: #FFFFFF`、`bgColor: #0B1220CC`、`padding: 10`。
- `bgColor: "none"` 可去掉底色。
- 文本最多 2000 字符。

### 5.7 高亮 `highlight`

```json
{"type":"highlight","x":80,"y":120,"w":400,"h":60,"color":"#FFD60A59"}
```

### 5.8 马赛克 `mosaic`

```json
{"type":"mosaic","x":300,"y":420,"w":500,"h":50,"blockSize":18}
```

- `blockSize` 默认 16，范围 2–64。
- 别名：`pixelate`。

### 5.9 序号徽章 `step`

```json
{"type":"step","x":960,"y":330,"index":1,"color":"#FF453A","size":32}
```

- `x,y` 是圆心。
- `index` 从 1 开始；前端手动放置时自动递增。
- `size` 是直径，默认 32。

### 限制

- 单次最多 200 个标注。
- 单条文字最多 2000 字符；画笔最多 5000 个点。
- 坐标允许超出图片，渲染时会裁剪。
- SnapMCP 仅输出 PNG。

---

## 6. 认证

默认不需要认证。若需要额外的本地保护，可在设置文件中配置：

`%APPDATA%\SnapMCP\settings.json`：

```json
{
  "shortcut": "CmdOrCtrl+Shift+X",
  "saveDir": "",
  "mcpEnabled": true,
  "mcpPort": 37621,
  "authToken": "your-long-random-token"
}
```

重启应用后，HTTP 请求必须携带以下任意一种头：

```http
Authorization: Bearer your-long-random-token
```

或：

```http
X-SnapMCP-Token: your-long-random-token
```

stdio 桥会自动从 `runtime.json` 读取 token 并附加请求头，Claude Desktop 无需额外配置。其他 HTTP 客户端需要自行配置认证头。

> `runtime.json` 包含明文 token（位于当前用户目录），请勿共享。

---

## 7. 错误语义

- **协议错误**（JSON-RPC）：`-32600`、`-32601`、`-32602`、`-32020`（HeaderMismatch）、`-32022`（UnsupportedProtocolVersion）。
- **工具错误**：HTTP 200 + `result.isError = true` + 文本内容。例如用户取消、路径不可写、没有上一次截图。
- **HTTP**：
  - `202`：通知已接受
  - `400`：JSON 解析失败、版本/头部不匹配
  - `401`：需要 token 或 token 错误
  - `403`：非本地 Origin
  - `404`：现代协议未知 method
  - `405`：GET `/mcp`

---

## 8. 冒烟测试

仓库自带一个 Node 脚本，测试 initialize、tools/list 与 list_displays：

```bash
npm run smoke:mcp
# 或指定地址
SNAPMCP_ENDPOINT=http://127.0.0.1:37621/mcp node scripts/smoke-mcp.mjs
```

它不依赖任何 MCP SDK，便于排查“客户端连不上但服务可能在运行”的问题。

---

## 9. 安全建议

1. 默认仅监听 loopback；不要用端口转发把它暴露到公网。
2. 如果本机运行了不受信任的网页或程序，建议开启 `authToken`。
3. `savePath` 可以写入任意用户可写的绝对路径；只把 MCP 接口给自己信任的 AI 客户端。
4. 截图属于高敏感数据；使用 `capture_interactive` 让人决定 AI 能看到什么。
5. 覆盖层关闭后临时文件会被删除，但 AI 工具保存的 PNG 会保留在截图目录，请自行管理。
