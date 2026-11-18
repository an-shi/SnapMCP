<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from "vue";
import Icon from "./components/Icon.vue";
import {
  captureNow,
  chooseSaveDir,
  copyText,
  getStatus,
  isTauri,
  listCaptures,
  openConfigDir,
  openSaveDir,
  quitApp,
  revealPath,
  setAutoStart,
  updateShortcut,
} from "./lib/bridge";
import type { AppStatus, CaptureEntry } from "./lib/types";

const status = ref<AppStatus | null>(null);
const captures = ref<CaptureEntry[]>([]);
const loading = ref(true);
const refreshing = ref(false);
const error = ref("");
const activeTab = ref<"stdio" | "http">("stdio");
const shortcutDraft = ref("");
const shortcutDirty = ref(false);
const shortcutSaving = ref(false);
const autoSaving = ref(false);
const quitArmed = ref(false);
const toast = ref<{ kind: "ok" | "error" | "info"; text: string } | null>(null);
let toastTimer: number | undefined;
let quitTimer: number | undefined;

const version = computed(() => status.value?.version ?? "0.1.0");
const running = computed(() => Boolean(status.value?.mcp.running));
const endpoint = computed(
  () => status.value?.mcp.endpoint || "http://127.0.0.1:37621/mcp",
);
const shortcutLabelText = computed(
  () => status.value?.shortcutLabel || "Ctrl + Shift + X",
);
const shortcutParts = computed(() =>
  shortcutLabelText.value.includes(" + ")
    ? shortcutLabelText.value.split(" + ")
    : [shortcutLabelText.value],
);
const mcpPort = computed(() =>
  status.value?.mcp.port ? String(status.value.mcp.port) : "37621",
);

const stdioSnippet = computed(() => {
  const exe = status.value?.appPath || "C:\\Program Files\\SnapMCP\\SnapMCP.exe";
  return JSON.stringify(
    {
      mcpServers: {
        snapmcp: {
          command: exe,
          args: ["--mcp-stdio"],
        },
      },
    },
    null,
    2,
  );
});

const httpSnippet = computed(() =>
  JSON.stringify(
    {
      mcpServers: {
        snapmcp: {
          type: "http",
          url: endpoint.value,
        },
      },
    },
    null,
    2,
  ),
);

const activeSnippet = computed(() =>
  activeTab.value === "stdio" ? stdioSnippet.value : httpSnippet.value,
);

const tools = [
  "take_screenshot",
  "annotate_screenshot",
  "capture_interactive",
  "list_displays",
];

const prompts = [
  {
    title: "标注截图",
    text: "截取当前屏幕，用红色箭头指向用户列表里的“删除”按钮，并在旁边写上“① 删除用户”。",
  },
  {
    title: "脱敏后发给 AI",
    text: "帮我截取主显示器，把邮箱地址打码，保存 PNG 后把路径发给我。",
  },
  {
    title: "人机协作",
    text: "请让用户自己框选报错弹窗并完成标注，然后把图片发给我分析。",
  },
];

function showToast(kind: "ok" | "error" | "info", text: string, timeout = 3200) {
  if (toastTimer) window.clearTimeout(toastTimer);
  toast.value = { kind, text };
  toastTimer = window.setTimeout(() => {
    toast.value = null;
  }, timeout);
}

function errorText(err: unknown): string {
  if (typeof err === "string") return err;
  if (err instanceof Error) return err.message;
  return JSON.stringify(err);
}

async function refresh(initial = false) {
  if (refreshing.value) return;
  refreshing.value = true;
  if (initial) loading.value = true;
  error.value = "";
  try {
    const [nextStatus, nextCaptures] = await Promise.all([
      getStatus(),
      listCaptures(8),
    ]);
    status.value = nextStatus;
    captures.value = nextCaptures;
    if (!shortcutDirty.value) shortcutDraft.value = nextStatus.shortcut;
  } catch (err) {
    error.value = errorText(err);
  } finally {
    loading.value = false;
    refreshing.value = false;
  }
}

async function copyValue(value: string, label: string) {
  try {
    await copyText(value);
    showToast("ok", `${label}已复制`);
  } catch (err) {
    showToast("error", `复制失败：${errorText(err)}`);
  }
}

async function applyShortcut() {
  if (!status.value || shortcutSaving.value) return;
  const next = shortcutDraft.value.trim();
  if (!next) {
    showToast("error", "快捷键不能为空");
    return;
  }
  shortcutSaving.value = true;
  try {
    await updateShortcut(next);
    shortcutDirty.value = false;
    await refresh();
    showToast("ok", "快捷键已更新");
  } catch (err) {
    showToast("error", errorText(err), 5200);
  } finally {
    shortcutSaving.value = false;
  }
}

function markShortcutDirty() {
  shortcutDirty.value = Boolean(
    status.value && shortcutDraft.value.trim() !== status.value.shortcut,
  );
}

function resetShortcutDraft() {
  if (!status.value) return;
  shortcutDraft.value = status.value.defaultShortcut;
  markShortcutDirty();
}

async function toggleAutoStart() {
  if (!status.value || autoSaving.value) return;
  const target = !status.value.autoStart;
  autoSaving.value = true;
  try {
    const actual = await setAutoStart(target);
    status.value.autoStart = actual;
    showToast("ok", actual ? "已开启开机自启" : "已关闭开机自启");
  } catch (err) {
    showToast("error", errorText(err));
  } finally {
    autoSaving.value = false;
  }
}

async function changeSaveDir() {
  try {
    const picked = await chooseSaveDir();
    if (picked) {
      showToast("ok", "截图保存目录已更新");
      await refresh();
    }
  } catch (err) {
    showToast("error", errorText(err));
  }
}

async function handleCapture() {
  try {
    await captureNow();
  } catch (err) {
    showToast("error", errorText(err), 5000);
  }
}

async function revealEntry(entry: CaptureEntry) {
  try {
    await revealPath(entry.path);
  } catch (err) {
    showToast("error", errorText(err));
  }
}

async function handleOpenSaveDir() {
  try {
    await openSaveDir();
  } catch (err) {
    showToast("error", errorText(err));
  }
}

async function handleOpenConfig() {
  try {
    await openConfigDir();
  } catch (err) {
    showToast("error", errorText(err));
  }
}

async function handleQuit() {
  if (!quitArmed.value) {
    quitArmed.value = true;
    if (quitTimer) window.clearTimeout(quitTimer);
    quitTimer = window.setTimeout(() => {
      quitArmed.value = false;
    }, 4000);
    return;
  }
  if (quitTimer) window.clearTimeout(quitTimer);
  try {
    await quitApp();
  } catch (err) {
    quitArmed.value = false;
    showToast("error", errorText(err));
  }
}

function relativeTime(ms: number): string {
  if (!ms) return "未知时间";
  const delta = Date.now() - ms;
  if (delta < 60_000) return "刚刚";
  if (delta < 3_600_000) return `${Math.floor(delta / 60_000)} 分钟前`;
  if (delta < 86_400_000) return `${Math.floor(delta / 3_600_000)} 小时前`;
  if (delta < 172_800_000) return "昨天";
  return new Date(ms).toLocaleDateString("zh-CN", {
    month: "short",
    day: "numeric",
  });
}

function formatSize(bytes: number): string {
  if (!bytes) return "0 KB";
  if (bytes < 1024 * 1024) return `${Math.round(bytes / 1024)} KB`;
  return `${(bytes / 1024 / 1024).toFixed(1)} MB`;
}

onMounted(() => {
  void refresh(true);
});

onBeforeUnmount(() => {
  if (toastTimer) window.clearTimeout(toastTimer);
  if (quitTimer) window.clearTimeout(quitTimer);
});
</script>

<template>
  <div class="app-shell">
    <div class="ambient" aria-hidden="true">
      <span class="glow glow-a" />
      <span class="glow glow-b" />
      <span class="grid-lines" />
    </div>

    <header class="app-header">
      <div class="brand">
        <div class="brand-logo">
          <Icon name="camera" :size="21" />
        </div>
        <div class="brand-copy">
          <div class="brand-name">
            SnapMCP
            <span class="version-pill">v{{ version }}</span>
          </div>
          <div class="brand-sub">截图 · 标注 · 给 AI 看</div>
        </div>
      </div>

      <div class="header-actions">
        <span v-if="!isTauri" class="preview-chip">浏览器预览</span>
        <span class="status-pill" :class="{ offline: !running }">
          <span class="status-dot" />
          {{ running ? "MCP 在线" : "MCP 离线" }}
          <span v-if="running" class="status-port">:{{ mcpPort }}</span>
        </span>
        <button
          type="button"
          class="icon-action"
          title="刷新状态"
          aria-label="刷新状态"
          @click="refresh()"
        >
          <Icon name="refresh" :size="16" :class="{ 'spin-once': refreshing }" />
        </button>
      </div>
    </header>

    <main class="content">
      <section class="card hero">
        <div class="hero-copy">
          <div class="eyebrow">
            <span class="eyebrow-line" />
            screenshot annotation for mcp
          </div>
          <h1>让每一步操作<br /><span>都有图可循。</span></h1>
          <p class="hero-lead">
            框选屏幕、画箭头、写序号，一键复制或交给 AI。轻量常驻托盘，
            内置标准 MCP Server，让 Claude 等客户端直接“看见”你的屏幕。
          </p>

          <div class="hero-actions">
            <button
              type="button"
              class="button primary capture-button"
              :disabled="!status"
              @click="handleCapture"
            >
              <Icon name="camera" :size="18" />
              截取屏幕
            </button>
            <div class="shortcut-display">
              <span class="shortcut-label">快捷键</span>
              <kbd v-for="part in shortcutParts" :key="part">{{ part }}</kbd>
            </div>
          </div>

          <div class="hero-meta">
            <span><Icon name="shield" :size="12" />纯本地运行</span>
            <span class="divider" />
            <span><Icon name="bolt" :size="12" />极低常驻开销</span>
            <span class="divider" />
            <span><Icon name="plug" :size="12" />MCP 原生</span>
          </div>
        </div>

        <div class="hero-visual" aria-hidden="true">
          <div class="fake-window">
            <div class="fake-titlebar">
              <span /><span /><span />
              <em>orbit admin — 用户管理</em>
            </div>
            <div class="fake-body">
              <div class="fake-sidebar">
                <i class="fake-logo" />
                <i v-for="n in 5" :key="n" :class="{ active: n === 2 }" />
                <i class="fake-user" />
              </div>
              <div class="fake-main">
                <div class="fake-head">
                  <span class="fake-h" />
                  <span class="fake-search" />
                  <span class="fake-avatar" />
                </div>
                <div class="fake-stat-row">
                  <span v-for="n in 3" :key="n" class="fake-stat" />
                </div>
                <div class="fake-table">
                  <div class="fake-table-head" />
                  <div v-for="n in 4" :key="n" class="fake-row" :class="{ danger: n === 3 }">
                    <span class="fake-avatar small" />
                    <span class="fake-line long" />
                    <span class="fake-line" />
                    <span v-if="n !== 3" class="fake-chip" />
                    <span v-else class="fake-delete">删除</span>
                  </div>
                </div>
                <svg class="fake-annotation" viewBox="0 0 420 260" fill="none">
                  <rect
                    x="307"
                    y="177"
                    width="56"
                    height="24"
                    rx="6"
                    stroke="#FF453A"
                    stroke-width="2"
                  />
                  <path
                    d="M240 230 C 272 220, 290 208, 316 195"
                    stroke="#FF453A"
                    stroke-width="2.4"
                    stroke-linecap="round"
                  />
                  <path d="M305 190 L 316 195 L 306 201 Z" fill="#FF453A" />
                  <circle cx="224" cy="235" r="11" fill="#FF453A" />
                  <text
                    x="224"
                    y="240"
                    text-anchor="middle"
                    fill="#FFFFFF"
                    font-size="11"
                    font-weight="700"
                  >
                    1
                  </text>
                </svg>
              </div>
            </div>
          </div>
          <div class="float-tag tag-tool">
            <Icon name="sparkles" :size="12" />
            take_screenshot
          </div>
          <div class="float-tag tag-copy">
            <Icon name="check" :size="12" />
            已复制 PNG
          </div>
          <div class="float-glow" />
        </div>
      </section>

      <div v-if="error" class="error-banner">
        <Icon name="info" :size="16" />
        <span>{{ error }}</span>
        <button type="button" @click="refresh()">重试</button>
      </div>

      <div v-if="status" class="dashboard-grid">
        <div class="column">
          <section class="card mcp-card">
            <div class="card-head">
              <div class="card-title-wrap">
                <span class="card-icon cyan"><Icon name="plug" :size="17" /></span>
                <div>
                  <h2>MCP 接口</h2>
                  <p>让 AI 客户端直接调用截图与标注能力</p>
                </div>
              </div>
              <span class="mini-pill" :class="{ ok: running }">
                <span class="status-dot" />
                {{ running ? "运行中" : "未连接" }}
              </span>
            </div>

            <div class="endpoint-row">
              <span class="endpoint-label">HTTP</span>
              <code>{{ endpoint }}</code>
              <button
                type="button"
                class="copy-btn"
                title="复制地址"
                aria-label="复制 MCP 地址"
                @click="copyValue(endpoint, 'MCP 地址')"
              >
                <Icon name="copy" :size="14" />
              </button>
            </div>

            <div
              v-if="status.mcp.error"
              class="inline-warning"
            >
              <Icon name="info" :size="13" />
              {{ status.mcp.error }}
            </div>
            <div v-if="status.mcp.tokenRequired" class="inline-warning token">
              <Icon name="shield" :size="13" />
              HTTP 接口已启用令牌校验，请在客户端附带 Authorization 头。
            </div>

            <div class="tabs" role="tablist" aria-label="MCP 接入方式">
              <button
                type="button"
                role="tab"
                :aria-selected="activeTab === 'stdio'"
                :class="{ active: activeTab === 'stdio' }"
                @click="activeTab = 'stdio'"
              >
                <Icon name="window" :size="13" />
                Claude Desktop · stdio
              </button>
              <button
                type="button"
                role="tab"
                :aria-selected="activeTab === 'http'"
                :class="{ active: activeTab === 'http' }"
                @click="activeTab = 'http'"
              >
                <Icon name="globe" :size="13" />
                Streamable HTTP
              </button>
            </div>

            <div class="code-block">
              <div class="code-bar">
                <span>{{ activeTab === "stdio" ? "claude_desktop_config.json" : "mcp.json" }}</span>
                <button
                  type="button"
                  class="copy-btn"
                  @click="copyValue(activeSnippet, '配置')"
                >
                  <Icon name="copy" :size="13" />
                  复制配置
                </button>
              </div>
              <pre><code>{{ activeSnippet }}</code></pre>
            </div>

            <p class="mcp-note">
              <template v-if="activeTab === 'stdio'">
                重启 Claude Desktop 后即可使用。SnapMCP 会自动保持单实例运行，无需手动启动。
              </template>
              <template v-else>
                适用于支持 Streamable HTTP 的 MCP 客户端；服务仅监听 127.0.0.1，不会暴露到局域网。
              </template>
            </p>

            <div class="tool-chips">
              <span class="tool-label">Tools</span>
              <code v-for="tool in tools" :key="tool">{{ tool }}</code>
            </div>
          </section>

          <section class="card settings-card">
            <div class="card-head">
              <div class="card-title-wrap">
                <span class="card-icon violet"><Icon name="settings" :size="17" /></span>
                <div>
                  <h2>偏好设置</h2>
                  <p>快捷键、自启动与输出位置</p>
                </div>
              </div>
            </div>

            <div class="setting-row">
              <div class="setting-label">
                <span class="setting-name">
                  <Icon name="keyboard" :size="14" />
                  全局截图快捷键
                </span>
                <p>示例：CmdOrCtrl+Shift+X 或 Alt+Shift+S</p>
                <p v-if="status.shortcutError" class="setting-error">
                  {{ status.shortcutError }}
                </p>
              </div>
              <div class="setting-control shortcut-control">
                <input
                  v-model="shortcutDraft"
                  class="text-field mono"
                  aria-label="快捷键"
                  spellcheck="false"
                  @input="markShortcutDirty"
                  @keydown.enter.prevent="applyShortcut"
                />
                <button
                  type="button"
                  class="small-btn primary-ghost"
                  :disabled="shortcutSaving || !shortcutDirty"
                  @click="applyShortcut"
                >
                  {{ shortcutSaving ? "应用中…" : "应用" }}
                </button>
                <button type="button" class="small-btn" @click="resetShortcutDraft">
                  默认
                </button>
              </div>
            </div>

            <div class="setting-row">
              <div class="setting-label">
                <span class="setting-name">
                  <Icon name="bolt" :size="14" />
                  开机自动启动
                </span>
                <p>登录系统后在托盘中静默运行，不弹窗</p>
              </div>
              <button
                type="button"
                class="switch"
                :class="{ on: status.autoStart }"
                role="switch"
                :aria-checked="status.autoStart"
                :disabled="autoSaving"
                @click="toggleAutoStart"
              >
                <span />
              </button>
            </div>

            <div class="setting-row">
              <div class="setting-label">
                <span class="setting-name">
                  <Icon name="folder" :size="14" />
                  截图保存目录
                </span>
                <p class="path-line">{{ status.saveDir }}</p>
              </div>
              <div class="setting-control">
                <button type="button" class="small-btn" @click="changeSaveDir">
                  更改
                </button>
                <button type="button" class="small-btn" @click="handleOpenSaveDir">
                  <Icon name="external" :size="12" />
                </button>
              </div>
            </div>

            <div class="setting-row">
              <div class="setting-label">
                <span class="setting-name">
                  <Icon name="shield" :size="14" />
                  配置与日志
                </span>
                <p class="path-line">{{ status.configDir }}</p>
              </div>
              <div class="setting-control">
                <button type="button" class="small-btn" @click="handleOpenConfig">
                  <Icon name="external" :size="12" />
                  打开
                </button>
              </div>
            </div>

            <div class="settings-footnote">
              <Icon name="info" :size="12" />
              关闭设置窗口后，SnapMCP 会继续在系统托盘运行。
            </div>
          </section>
        </div>

        <div class="column">
          <section class="card recent-card">
            <div class="card-head">
              <div class="card-title-wrap">
                <span class="card-icon amber"><Icon name="image" :size="17" /></span>
                <div>
                  <h2>最近截图</h2>
                  <p>{{ status.captureCount }} 张 PNG 保存在输出目录</p>
                </div>
              </div>
              <button
                type="button"
                class="icon-action subtle"
                title="打开截图文件夹"
                aria-label="打开截图文件夹"
                @click="handleOpenSaveDir"
              >
                <Icon name="folder" :size="15" />
              </button>
            </div>

            <div v-if="captures.length" class="capture-list">
              <button
                v-for="entry in captures"
                :key="entry.path"
                type="button"
                class="capture-row"
                :title="entry.path"
                @click="revealEntry(entry)"
              >
                <span class="file-glyph">
                  <Icon name="image" :size="15" />
                </span>
                <span class="capture-meta">
                  <span class="capture-name">{{ entry.name }}</span>
                  <span class="capture-sub">
                    {{ relativeTime(entry.modifiedMs) }} · {{ formatSize(entry.size) }}
                  </span>
                </span>
                <Icon name="external" :size="13" class="row-arrow" />
              </button>
            </div>
            <div v-else class="empty-state">
              <span class="empty-icon"><Icon name="camera" :size="22" /></span>
              <strong>还没有截图</strong>
              <p>按 {{ shortcutLabelText }} 或点击上方“截取屏幕”开始。</p>
            </div>
          </section>

          <section class="card examples-card">
            <div class="card-head">
              <div class="card-title-wrap">
                <span class="card-icon green"><Icon name="sparkles" :size="17" /></span>
                <div>
                  <h2>对 AI 这样说</h2>
                  <p>复制示例到 Claude / Cursor / 其他 MCP 客户端</p>
                </div>
              </div>
            </div>
            <div class="example-list">
              <div v-for="(prompt, index) in prompts" :key="prompt.title" class="example">
                <div class="example-head">
                  <span class="example-index">0{{ index + 1 }}</span>
                  <span class="example-title">{{ prompt.title }}</span>
                  <button
                    type="button"
                    class="icon-action tiny"
                    title="复制这句话"
                    aria-label="复制提示词"
                    @click="copyValue(prompt.text, '提示词')"
                  >
                    <Icon name="copy" :size="12" />
                  </button>
                </div>
                <p>{{ prompt.text }}</p>
              </div>
            </div>
            <div class="examples-foot">
              <Icon name="shield" :size="12" />
              每张截图都由你主动框选或确认；AI 无法在无感知时读取其他窗口。
            </div>
          </section>

          <section class="card quick-card">
            <div class="quick-line">
              <span class="card-icon slate"><Icon name="crosshair" :size="16" /></span>
              <div>
                <strong>快捷操作</strong>
                <p>截图后回车复制，Ctrl+S 快速保存；<kbd>Esc</kbd> 随时退出。</p>
              </div>
            </div>
            <div class="quick-actions">
              <button type="button" class="small-btn" @click="handleCapture">
                <Icon name="camera" :size="12" />
                立即截图
              </button>
              <button type="button" class="small-btn" @click="handleOpenConfig">
                <Icon name="folder" :size="12" />
                配置目录
              </button>
            </div>
          </section>
        </div>
      </div>

      <div v-else-if="loading" class="loading-block">
        <span class="loader" />
        <p>正在读取 SnapMCP 状态…</p>
      </div>

      <footer class="app-footer">
        <div class="footer-left">
          <span>SnapMCP v{{ version }}</span>
          <span class="divider" />
          <span>{{ isTauri ? "桌面应用 · 托盘常驻" : "浏览器预览模式" }}</span>
          <span class="divider" />
          <span class="muted">MCP {{ running ? "online" : "offline" }}</span>
        </div>
        <div class="footer-actions">
          <button type="button" class="footer-btn" @click="handleOpenConfig">
            <Icon name="shield" :size="12" />
            配置目录
          </button>
          <button
            type="button"
            class="footer-btn danger"
            :class="{ armed: quitArmed }"
            @click="handleQuit"
          >
            <Icon name="close" :size="12" />
            {{ quitArmed ? "再次点击确认退出" : "退出 SnapMCP" }}
          </button>
        </div>
      </footer>
    </main>

    <Transition name="toast">
      <div v-if="toast" class="app-toast" :class="toast.kind">
        <Icon
          :name="toast.kind === 'error' ? 'info' : toast.kind === 'ok' ? 'check' : 'dot'"
          :size="15"
        />
        <span>{{ toast.text }}</span>
      </div>
    </Transition>
  </div>
</template>

<style scoped>
.app-shell {
  position: relative;
  height: 100vh;
  overflow-y: auto;
  overflow-x: hidden;
  background: #060a11;
  color: var(--text-1);
}

.ambient {
  position: fixed;
  inset: 0;
  z-index: 0;
  pointer-events: none;
  overflow: hidden;
}

.glow {
  position: absolute;
  border-radius: 50%;
  filter: blur(110px);
  opacity: 0.22;
}

.glow-a {
  width: 520px;
  height: 440px;
  top: -160px;
  left: 12%;
  background: #0a84ff;
}

.glow-b {
  width: 420px;
  height: 380px;
  top: 20px;
  right: -120px;
  background: #22d3ee;
  opacity: 0.13;
}

.grid-lines {
  position: absolute;
  inset: 0;
  opacity: 0.16;
  background-image:
    linear-gradient(rgba(129, 168, 199, 0.14) 1px, transparent 1px),
    linear-gradient(90deg, rgba(129, 168, 199, 0.14) 1px, transparent 1px);
  background-size: 46px 46px;
  mask-image: radial-gradient(ellipse 82% 55% at 50% 0%, black 5%, transparent 74%);
}

.app-header {
  position: relative;
  z-index: 3;
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 18px;
  height: 66px;
  padding: 0 30px;
  border-bottom: 1px solid rgba(148, 178, 209, 0.09);
  background: rgba(6, 10, 17, 0.72);
  backdrop-filter: blur(18px);
  position: sticky;
  top: 0;
}

.brand {
  display: flex;
  align-items: center;
  gap: 11px;
}

.brand-logo {
  width: 35px;
  height: 35px;
  display: grid;
  place-items: center;
  border-radius: 10px;
  color: #04141d;
  background: linear-gradient(145deg, #67e8f9, #0a84ff 84%);
  box-shadow:
    0 8px 22px rgba(34, 211, 238, 0.22),
    0 1px 0 rgba(255, 255, 255, 0.5) inset;
}

.brand-name {
  display: flex;
  align-items: center;
  gap: 8px;
  color: #f2f9ff;
  font-size: 15px;
  font-weight: 700;
  letter-spacing: 0.01em;
  line-height: 1.1;
}

.version-pill {
  padding: 2px 6px;
  border: 1px solid rgba(148, 178, 209, 0.14);
  border-radius: 5px;
  color: #657c94;
  font-family: var(--font-mono);
  font-size: 8px;
  font-weight: 500;
}

.brand-sub {
  margin-top: 3px;
  color: #61758b;
  font-size: 9.5px;
  letter-spacing: 0.04em;
}

.header-actions {
  display: flex;
  align-items: center;
  gap: 9px;
}

.preview-chip {
  padding: 5px 8px;
  border: 1px solid rgba(255, 214, 10, 0.18);
  border-radius: 6px;
  background: rgba(255, 214, 10, 0.06);
  color: #d8b94a;
  font-size: 9.5px;
  font-weight: 600;
}

.status-pill {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  height: 27px;
  padding: 0 10px;
  border: 1px solid rgba(50, 215, 75, 0.17);
  border-radius: 99px;
  background: rgba(50, 215, 75, 0.055);
  color: #8ce1a1;
  font-size: 10px;
  font-weight: 600;
  white-space: nowrap;
}

.status-pill.offline {
  border-color: rgba(255, 255, 255, 0.09);
  background: rgba(255, 255, 255, 0.03);
  color: #7c8ea3;
}

.status-dot {
  width: 6px;
  height: 6px;
  flex: none;
  border-radius: 50%;
  background: #32d74b;
  box-shadow: 0 0 0 3px rgba(50, 215, 75, 0.1);
}

.status-pill.offline .status-dot {
  background: #64748b;
  box-shadow: none;
}

.status-port {
  color: #4fae68;
  font-family: var(--font-mono);
  font-weight: 500;
}

.icon-action {
  width: 30px;
  height: 30px;
  display: grid;
  place-items: center;
  border: 1px solid rgba(148, 178, 209, 0.11);
  border-radius: 8px;
  background: rgba(255, 255, 255, 0.025);
  color: #7d91a8;
  cursor: pointer;
  transition:
    color 0.15s ease,
    background 0.15s ease,
    border-color 0.15s ease;
}

.icon-action:hover {
  color: #eaf7ff;
  background: rgba(255, 255, 255, 0.06);
  border-color: rgba(148, 178, 209, 0.2);
}

.icon-action.subtle {
  border-color: transparent;
  background: transparent;
}

.icon-action.tiny {
  width: 23px;
  height: 23px;
  border-radius: 6px;
}

.spin-once {
  animation: spin 0.7s linear infinite;
}

@keyframes spin {
  to {
    transform: rotate(360deg);
  }
}

.content {
  position: relative;
  z-index: 2;
  width: min(1180px, 100%);
  margin: 0 auto;
  padding: 26px 30px 30px;
}

.card {
  position: relative;
  border: 1px solid rgba(148, 178, 209, 0.105);
  border-radius: 18px;
  background:
    linear-gradient(180deg, rgba(18, 29, 43, 0.6), rgba(9, 15, 24, 0.85));
  box-shadow:
    0 24px 60px rgba(0, 0, 0, 0.22),
    0 1px 0 rgba(255, 255, 255, 0.035) inset;
  backdrop-filter: blur(6px);
}

.hero {
  display: grid;
  grid-template-columns: minmax(0, 1.06fr) minmax(340px, 0.94fr);
  gap: 18px;
  min-height: 330px;
  padding: 34px 36px;
  overflow: hidden;
  background:
    radial-gradient(ellipse 50% 80% at 88% 22%, rgba(34, 211, 238, 0.14), transparent 68%),
    radial-gradient(ellipse 34% 58% at 8% 100%, rgba(10, 132, 255, 0.12), transparent 70%),
    linear-gradient(112deg, rgba(17, 30, 46, 0.96), rgba(9, 17, 28, 0.96));
  animation: fade-up 0.46s cubic-bezier(0.16, 0.84, 0.24, 1) both;
}

.hero::before {
  content: "";
  position: absolute;
  inset: 0;
  border-radius: inherit;
  background-image: radial-gradient(rgba(148, 190, 220, 0.15) 0.6px, transparent 0.6px);
  background-size: 18px 18px;
  opacity: 0.2;
  mask-image: radial-gradient(ellipse 45% 70% at 20% 90%, black, transparent 75%);
  pointer-events: none;
}

.hero-copy {
  position: relative;
  z-index: 2;
  padding-top: 4px;
}

.eyebrow {
  display: inline-flex;
  align-items: center;
  gap: 8px;
  color: #69dce9;
  font-family: var(--font-mono);
  font-size: 8px;
  font-weight: 600;
  letter-spacing: 0.16em;
  text-transform: uppercase;
}

.eyebrow-line {
  width: 18px;
  height: 1px;
  background: linear-gradient(90deg, #22d3ee, transparent);
}

.hero h1 {
  margin: 15px 0 13px;
  color: #f3f9ff;
  font-size: clamp(30px, 3.4vw, 42px);
  font-weight: 700;
  line-height: 1.16;
  letter-spacing: -0.035em;
}

.hero h1 span {
  background: linear-gradient(92deg, #8fefff 0%, #5cc8ff 52%, #7f9cff 100%);
  -webkit-background-clip: text;
  background-clip: text;
  color: transparent;
}

.hero-lead {
  max-width: 530px;
  margin: 0;
  color: #8ea4ba;
  font-size: 12.5px;
  line-height: 1.75;
}

.hero-actions {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 14px;
  margin-top: 25px;
}

.button {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 8px;
  height: 40px;
  padding: 0 17px;
  border-radius: 10px;
  border: 1px solid transparent;
  font-family: inherit;
  font-size: 11.5px;
  font-weight: 700;
  cursor: pointer;
  transition:
    transform 0.16s ease,
    filter 0.16s ease,
    background 0.16s ease;
}

.button.primary {
  color: #04141d;
  background: linear-gradient(180deg, #6be8f4, #22d3ee);
  border-color: rgba(157, 247, 255, 0.48);
  box-shadow:
    0 10px 26px rgba(34, 211, 238, 0.2),
    0 1px 0 rgba(255, 255, 255, 0.5) inset;
}

.button.primary:hover:not(:disabled) {
  filter: brightness(1.055);
  transform: translateY(-1px);
}

.button.primary:disabled {
  opacity: 0.55;
  cursor: default;
}

.capture-button {
  min-width: 132px;
}

.shortcut-display {
  display: flex;
  align-items: center;
  gap: 5px;
}

.shortcut-label {
  margin-right: 3px;
  color: #63798f;
  font-size: 9px;
}

.shortcut-display kbd {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  min-width: 25px;
  height: 25px;
  padding: 0 7px;
  border: 1px solid rgba(148, 178, 209, 0.16);
  border-bottom-color: rgba(148, 178, 209, 0.25);
  border-radius: 6px;
  background: rgba(255, 255, 255, 0.04);
  color: #b9cee3;
  font-family: var(--font-mono);
  font-size: 8.5px;
  font-weight: 500;
  box-shadow: 0 2px 0 rgba(0, 0, 0, 0.18);
}

.hero-meta {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 9px;
  margin-top: 22px;
  color: #60768c;
  font-size: 9.5px;
}

.hero-meta span {
  display: inline-flex;
  align-items: center;
  gap: 5px;
}

.hero-meta .divider {
  width: 2px;
  height: 2px;
  border-radius: 50%;
  background: #35495d;
}

.hero-visual {
  position: relative;
  z-index: 1;
  display: flex;
  align-items: center;
  justify-content: center;
  min-width: 0;
}

.float-glow {
  position: absolute;
  width: 280px;
  height: 230px;
  border-radius: 50%;
  background: rgba(34, 211, 238, 0.13);
  filter: blur(60px);
  z-index: 0;
}

.fake-window {
  position: relative;
  z-index: 2;
  width: min(100%, 405px);
  aspect-ratio: 1.55;
  overflow: hidden;
  border: 1px solid rgba(160, 200, 225, 0.22);
  border-radius: 12px;
  background: #f4f7fb;
  box-shadow:
    0 32px 70px rgba(0, 0, 0, 0.46),
    0 0 0 6px rgba(8, 15, 25, 0.32);
  transform: perspective(900px) rotateY(-4deg) rotateX(1deg);
  transform-origin: center right;
}

.fake-titlebar {
  height: 25px;
  display: flex;
  align-items: center;
  gap: 4px;
  padding: 0 9px;
  background: #0f1b2d;
}

.fake-titlebar > span {
  width: 4px;
  height: 4px;
  border-radius: 50%;
  background: #40536a;
}

.fake-titlebar em {
  margin-left: auto;
  color: #5f748b;
  font-family: var(--font-mono);
  font-size: 5px;
  font-style: normal;
  letter-spacing: 0.04em;
}

.fake-body {
  display: flex;
  height: calc(100% - 25px);
}

.fake-sidebar {
  width: 52px;
  flex: none;
  padding: 10px 8px;
  background: #0b1524;
}

.fake-sidebar i {
  display: block;
  height: 4px;
  margin-bottom: 8px;
  border-radius: 99px;
  background: #25364b;
}

.fake-sidebar i.fake-logo {
  width: 18px;
  height: 18px;
  margin-bottom: 14px;
  border-radius: 5px;
  background: linear-gradient(145deg, #22d3ee, #0a84ff);
}

.fake-sidebar i.active {
  background: #2f6c82;
}

.fake-sidebar i.fake-user {
  width: 18px;
  height: 18px;
  margin-top: 58px;
  border-radius: 50%;
  background: #31465f;
}

.fake-main {
  position: relative;
  flex: 1;
  padding: 10px 12px;
  background: #f5f8fb;
}

.fake-head {
  display: flex;
  align-items: center;
  gap: 7px;
  height: 16px;
  margin-bottom: 9px;
}

.fake-h {
  width: 62px;
  height: 7px;
  border-radius: 99px;
  background: #dae3ee;
}

.fake-search {
  width: 95px;
  height: 12px;
  margin-left: auto;
  border-radius: 99px;
  background: #e7edf4;
}

.fake-avatar {
  width: 12px;
  height: 12px;
  border-radius: 50%;
  background: #cbd8e6;
}

.fake-avatar.small {
  width: 10px;
  height: 10px;
  flex: none;
  background: #d4e1ef;
}

.fake-stat-row {
  display: flex;
  gap: 6px;
  margin-bottom: 8px;
}

.fake-stat {
  flex: 1;
  height: 34px;
  border: 1px solid #e9eef4;
  border-radius: 5px;
  background: #ffffff;
}

.fake-table {
  border: 1px solid #e8edf3;
  border-radius: 6px;
  overflow: hidden;
  background: #ffffff;
}

.fake-table-head {
  height: 13px;
  background: #f9fbfd;
  border-bottom: 1px solid #edf1f6;
}

.fake-row {
  position: relative;
  display: flex;
  align-items: center;
  gap: 7px;
  height: 28px;
  padding: 0 8px;
  border-bottom: 1px solid #f0f4f8;
}

.fake-row:last-child {
  border-bottom: 0;
}

.fake-line {
  width: 30px;
  height: 4px;
  border-radius: 99px;
  background: #e5ebf2;
}

.fake-line.long {
  width: 50px;
  background: #d8e1eb;
}

.fake-chip {
  width: 25px;
  height: 8px;
  margin-left: auto;
  border-radius: 99px;
  background: #eaf8ef;
}

.fake-row.danger .fake-line {
  opacity: 0.55;
}

.fake-delete {
  margin-left: auto;
  padding: 2.5px 6px;
  border: 1px solid #ffd7d3;
  border-radius: 4px;
  background: #fff1f0;
  color: #e11d48;
  font-size: 4.5px;
  font-weight: 700;
}

.fake-annotation {
  position: absolute;
  inset: 0;
  width: 100%;
  height: 100%;
  pointer-events: none;
}

.float-tag {
  position: absolute;
  z-index: 4;
  display: inline-flex;
  align-items: center;
  gap: 5px;
  padding: 6px 9px;
  border: 1px solid rgba(148, 198, 225, 0.2);
  border-radius: 8px;
  background: rgba(8, 16, 26, 0.9);
  color: #b9e6ef;
  font-family: var(--font-mono);
  font-size: 8px;
  box-shadow: 0 10px 30px rgba(0, 0, 0, 0.35);
  backdrop-filter: blur(10px);
}

.tag-tool {
  top: 4px;
  right: 8px;
}

.tag-tool :deep(svg) {
  color: #67e8f9;
}

.tag-copy {
  bottom: 10px;
  left: 2px;
  color: #a8f3c1;
  border-color: rgba(50, 215, 75, 0.2);
}

.tag-copy :deep(svg) {
  color: #32d74b;
}

.error-banner {
  display: flex;
  align-items: center;
  gap: 9px;
  margin-top: 16px;
  padding: 10px 13px;
  border: 1px solid rgba(255, 69, 58, 0.2);
  border-radius: 10px;
  background: rgba(255, 69, 58, 0.055);
  color: #ffaaa4;
  font-size: 10px;
}

.error-banner button {
  margin-left: auto;
  padding: 4px 9px;
  border: 1px solid rgba(255, 138, 130, 0.25);
  border-radius: 6px;
  background: transparent;
  color: #ffc9c5;
  font-size: 9px;
  cursor: pointer;
}

.dashboard-grid {
  display: grid;
  grid-template-columns: minmax(0, 1.2fr) minmax(310px, 0.8fr);
  gap: 16px;
  margin-top: 16px;
  align-items: start;
}

.column {
  display: flex;
  flex-direction: column;
  gap: 16px;
  min-width: 0;
}

.dashboard-grid > .column:nth-child(1) > .card {
  animation: fade-up 0.46s 0.05s cubic-bezier(0.16, 0.84, 0.24, 1) both;
}

.dashboard-grid > .column:nth-child(2) > .card {
  animation: fade-up 0.46s 0.1s cubic-bezier(0.16, 0.84, 0.24, 1) both;
}

@keyframes fade-up {
  from {
    opacity: 0;
    transform: translateY(10px);
  }
  to {
    opacity: 1;
    transform: translateY(0);
  }
}

.card-head {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 12px;
}

.card-title-wrap {
  display: flex;
  align-items: center;
  gap: 10px;
}

.card-icon {
  width: 32px;
  height: 32px;
  display: grid;
  place-items: center;
  flex: none;
  border-radius: 9px;
  border: 1px solid rgba(255, 255, 255, 0.06);
}

.card-icon.cyan {
  color: #67e8f9;
  background: rgba(34, 211, 238, 0.08);
}

.card-icon.violet {
  color: #c4a7ff;
  background: rgba(191, 90, 242, 0.08);
}

.card-icon.amber {
  color: #ffd66b;
  background: rgba(255, 214, 10, 0.075);
}

.card-icon.green {
  color: #78e5a0;
  background: rgba(50, 215, 75, 0.07);
}

.card-icon.slate {
  color: #9fb7cd;
  background: rgba(148, 178, 209, 0.06);
}

.card h2 {
  margin: 0;
  color: #eaf4ff;
  font-size: 13.5px;
  font-weight: 700;
  letter-spacing: 0.005em;
}

.card-title-wrap p {
  margin: 3px 0 0;
  color: #667d93;
  font-size: 10px;
  line-height: 1.45;
}

.mini-pill {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  height: 22px;
  padding: 0 8px;
  flex: none;
  border: 1px solid rgba(255, 255, 255, 0.08);
  border-radius: 99px;
  color: #7f91a5;
  font-size: 9px;
  font-weight: 600;
}

.mini-pill.ok {
  border-color: rgba(50, 215, 75, 0.16);
  background: rgba(50, 215, 75, 0.045);
  color: #8bdfa0;
}

.mcp-card,
.settings-card,
.recent-card,
.examples-card,
.quick-card {
  padding: 19px 20px;
}

.endpoint-row {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-top: 15px;
  padding: 7px 7px 7px 10px;
  border: 1px solid rgba(148, 178, 209, 0.1);
  border-radius: 9px;
  background: rgba(3, 8, 14, 0.55);
}

.endpoint-label {
  padding: 2px 5px;
  flex: none;
  border-radius: 4px;
  background: rgba(34, 211, 238, 0.1);
  color: #58c9da;
  font-family: var(--font-mono);
  font-size: 8px;
  font-weight: 600;
  letter-spacing: 0.06em;
}

.endpoint-row code {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  color: #a8c0d6;
  font-family: var(--font-mono);
  font-size: 10.5px;
}

.copy-btn {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  height: 25px;
  padding: 0 8px;
  flex: none;
  border: 1px solid rgba(148, 178, 209, 0.11);
  border-radius: 6px;
  background: rgba(255, 255, 255, 0.035);
  color: #7f96ad;
  font-family: inherit;
  font-size: 9px;
  cursor: pointer;
  transition:
    color 0.15s ease,
    background 0.15s ease;
}

.copy-btn:hover {
  color: #dff8ff;
  background: rgba(34, 211, 238, 0.08);
}

.inline-warning {
  display: flex;
  align-items: flex-start;
  gap: 7px;
  margin-top: 9px;
  padding: 8px 10px;
  border: 1px solid rgba(255, 214, 10, 0.15);
  border-radius: 7px;
  background: rgba(255, 214, 10, 0.045);
  color: #cbae58;
  font-size: 9.5px;
  line-height: 1.5;
}

.inline-warning.token {
  border-color: rgba(34, 211, 238, 0.15);
  background: rgba(34, 211, 238, 0.04);
  color: #74b7c3;
}

.tabs {
  display: flex;
  gap: 3px;
  margin-top: 14px;
  padding: 3px;
  border: 1px solid rgba(148, 178, 209, 0.075);
  border-radius: 8px;
  background: rgba(3, 8, 14, 0.36);
}

.tabs button {
  flex: 1;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 6px;
  height: 27px;
  border: 1px solid transparent;
  border-radius: 6px;
  background: transparent;
  color: #6e849a;
  font-family: inherit;
  font-size: 9.5px;
  font-weight: 600;
  white-space: nowrap;
  cursor: pointer;
  transition:
    color 0.15s ease,
    background 0.15s ease,
    border-color 0.15s ease;
}

.tabs button:hover {
  color: #bcd3e7;
}

.tabs button.active {
  color: #dffaff;
  border-color: rgba(34, 211, 238, 0.14);
  background: rgba(34, 211, 238, 0.075);
}

.code-block {
  margin-top: 8px;
  overflow: hidden;
  border: 1px solid rgba(148, 178, 209, 0.09);
  border-radius: 9px;
  background: #050a11;
}

.code-bar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  height: 28px;
  padding: 0 7px 0 11px;
  border-bottom: 1px solid rgba(148, 178, 209, 0.07);
  background: rgba(255, 255, 255, 0.022);
}

.code-bar > span {
  color: #576d84;
  font-family: var(--font-mono);
  font-size: 7.5px;
  letter-spacing: 0.02em;
}

.code-bar .copy-btn {
  height: 21px;
  border-color: transparent;
  background: transparent;
  font-size: 7.5px;
}

.code-block pre {
  max-height: 160px;
  margin: 0;
  padding: 11px 12px;
  overflow: auto;
  color: #94b3cb;
  font-family: var(--font-mono);
  font-size: 9.5px;
  line-height: 1.65;
  white-space: pre;
}

.code-block code {
  font-family: inherit;
}

.mcp-note {
  margin: 9px 0 0;
  color: #5f748a;
  font-size: 9.5px;
  line-height: 1.6;
}

.tool-chips {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 5px;
  margin-top: 12px;
  padding-top: 11px;
  border-top: 1px solid rgba(148, 178, 209, 0.075);
}

.tool-label {
  margin-right: 2px;
  color: #4f6479;
  font-family: var(--font-mono);
  font-size: 7px;
  letter-spacing: 0.09em;
  text-transform: uppercase;
}

.tool-chips code {
  padding: 3px 6px;
  border: 1px solid rgba(148, 178, 209, 0.075);
  border-radius: 5px;
  background: rgba(255, 255, 255, 0.024);
  color: #7f9ab1;
  font-family: var(--font-mono);
  font-size: 8px;
}

.setting-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
  padding: 13px 0;
  border-bottom: 1px solid rgba(148, 178, 209, 0.075);
}

.setting-row:last-of-type {
  border-bottom: 0;
  padding-bottom: 5px;
}

.setting-label {
  min-width: 0;
}

.setting-name {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  color: #c3d5e6;
  font-size: 11px;
  font-weight: 600;
}

.setting-name :deep(svg) {
  color: #69839b;
}

.setting-label p {
  margin: 4px 0 0;
  color: #5b7086;
  font-size: 9.5px;
  line-height: 1.5;
}

.setting-label .setting-error {
  color: #e79c96;
}

.setting-label .path-line {
  max-width: 340px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-family: var(--font-mono);
  font-size: 9px;
  color: #728aa0;
}

.setting-control {
  display: flex;
  align-items: center;
  gap: 5px;
  flex: none;
}

.shortcut-control {
  flex-wrap: wrap;
  justify-content: flex-end;
}

.text-field {
  width: 185px;
  height: 30px;
  padding: 0 9px;
  border: 1px solid rgba(148, 178, 209, 0.13);
  border-radius: 7px;
  background: rgba(3, 8, 14, 0.55);
  color: #bcd3e6;
  font-family: var(--font-mono);
  font-size: 9.5px;
  outline: none;
  transition:
    border-color 0.15s ease,
    box-shadow 0.15s ease;
}

.text-field:focus {
  border-color: rgba(34, 211, 238, 0.4);
  box-shadow: 0 0 0 3px rgba(34, 211, 238, 0.055);
}

.small-btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 4px;
  height: 28px;
  padding: 0 10px;
  border: 1px solid rgba(148, 178, 209, 0.12);
  border-radius: 7px;
  background: rgba(255, 255, 255, 0.03);
  color: #91a8bd;
  font-family: inherit;
  font-size: 9.5px;
  font-weight: 600;
  white-space: nowrap;
  cursor: pointer;
  transition:
    color 0.15s ease,
    background 0.15s ease,
    border-color 0.15s ease;
}

.small-btn:hover:not(:disabled) {
  color: #e8f7ff;
  background: rgba(255, 255, 255, 0.06);
  border-color: rgba(148, 178, 209, 0.2);
}

.small-btn:disabled {
  opacity: 0.4;
  cursor: default;
}

.small-btn.primary-ghost {
  color: #77e7f3;
  border-color: rgba(34, 211, 238, 0.2);
  background: rgba(34, 211, 238, 0.07);
}

.switch {
  width: 37px;
  height: 21px;
  flex: none;
  padding: 2px;
  border: 1px solid rgba(148, 178, 209, 0.14);
  border-radius: 99px;
  background: rgba(255, 255, 255, 0.055);
  cursor: pointer;
  transition:
    background 0.18s ease,
    border-color 0.18s ease;
}

.switch span {
  display: block;
  width: 15px;
  height: 15px;
  border-radius: 50%;
  background: #7b8ea3;
  transition:
    transform 0.18s cubic-bezier(0.2, 0.8, 0.2, 1),
    background 0.18s ease;
}

.switch.on {
  border-color: rgba(34, 211, 238, 0.32);
  background: rgba(34, 211, 238, 0.18);
}

.switch.on span {
  transform: translateX(16px);
  background: #67e8f9;
  box-shadow: 0 0 8px rgba(34, 211, 238, 0.35);
}

.switch:disabled {
  opacity: 0.5;
  cursor: default;
}

.settings-footnote {
  display: flex;
  align-items: center;
  gap: 6px;
  margin-top: 11px;
  color: #53697e;
  font-size: 9px;
}

.recent-card .card-head,
.examples-card .card-head {
  margin-bottom: 11px;
}

.capture-list {
  display: flex;
  flex-direction: column;
  gap: 2px;
  max-height: 238px;
  overflow-y: auto;
  margin: 0 -5px;
  padding: 0 5px;
}

.capture-row {
  display: flex;
  align-items: center;
  gap: 9px;
  width: 100%;
  padding: 7px 6px;
  border: 1px solid transparent;
  border-radius: 8px;
  background: transparent;
  text-align: left;
  cursor: pointer;
  transition:
    background 0.14s ease,
    border-color 0.14s ease;
}

.capture-row:hover {
  border-color: rgba(148, 178, 209, 0.08);
  background: rgba(255, 255, 255, 0.032);
}

.file-glyph {
  width: 28px;
  height: 28px;
  display: grid;
  place-items: center;
  flex: none;
  border: 1px solid rgba(148, 178, 209, 0.08);
  border-radius: 7px;
  background: rgba(255, 255, 255, 0.025);
  color: #7f9bb4;
}

.capture-meta {
  display: flex;
  flex-direction: column;
  gap: 2px;
  min-width: 0;
  flex: 1;
}

.capture-name {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  color: #afc5d9;
  font-family: var(--font-mono);
  font-size: 9.5px;
}

.capture-sub {
  color: #586d82;
  font-size: 9px;
}

.row-arrow {
  flex: none;
  color: #4e6378;
  opacity: 0;
  transition: opacity 0.14s ease;
}

.capture-row:hover .row-arrow {
  opacity: 1;
}

.empty-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: 24px 16px 18px;
  text-align: center;
}

.empty-icon {
  width: 44px;
  height: 44px;
  display: grid;
  place-items: center;
  margin-bottom: 10px;
  border: 1px dashed rgba(148, 178, 209, 0.16);
  border-radius: 12px;
  color: #50677d;
  background: rgba(255, 255, 255, 0.018);
}

.empty-state strong {
  color: #869cb2;
  font-size: 10px;
  font-weight: 600;
}

.empty-state p {
  margin: 5px 0 0;
  color: #556b80;
  font-size: 9.5px;
  line-height: 1.55;
}

.example-list {
  display: flex;
  flex-direction: column;
  gap: 7px;
}

.example {
  padding: 9px 10px;
  border: 1px solid rgba(148, 178, 209, 0.08);
  border-radius: 8px;
  background: rgba(255, 255, 255, 0.018);
}

.example-head {
  display: flex;
  align-items: center;
  gap: 6px;
}

.example-index {
  color: #4f6a83;
  font-family: var(--font-mono);
  font-size: 7px;
}

.example-title {
  color: #89a3ba;
  font-size: 9.5px;
  font-weight: 600;
}

.example-head .icon-action {
  margin-left: auto;
}

.example p {
  margin: 5px 0 0;
  color: #6d8298;
  font-size: 9.5px;
  line-height: 1.62;
}

.examples-foot {
  display: flex;
  align-items: flex-start;
  gap: 6px;
  margin-top: 11px;
  color: #4f6478;
  font-size: 8.5px;
  line-height: 1.5;
}

.quick-card {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 14px;
  flex-wrap: wrap;
  background:
    linear-gradient(120deg, rgba(17, 29, 43, 0.7), rgba(9, 15, 24, 0.7));
}

.quick-line {
  display: flex;
  align-items: center;
  gap: 10px;
  min-width: 0;
}

.quick-line strong {
  display: block;
  color: #b5cbe0;
  font-size: 10.5px;
  font-weight: 600;
}

.quick-line p {
  margin: 3px 0 0;
  color: #5e748a;
  font-size: 9.5px;
  line-height: 1.5;
}

.quick-line kbd {
  padding: 1px 4px;
  border: 1px solid rgba(148, 178, 209, 0.14);
  border-radius: 4px;
  font-family: var(--font-mono);
  font-size: 7px;
}

.quick-actions {
  display: flex;
  gap: 5px;
}

.loading-block {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  min-height: 260px;
  color: #657d93;
  font-size: 10px;
}

.loader {
  width: 22px;
  height: 22px;
  margin-bottom: 12px;
  border: 2px solid rgba(34, 211, 238, 0.14);
  border-top-color: #22d3ee;
  border-radius: 50%;
  animation: spin 0.8s linear infinite;
}

.app-footer {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 14px;
  flex-wrap: wrap;
  margin-top: 22px;
  padding: 14px 2px 2px;
  border-top: 1px solid rgba(148, 178, 209, 0.075);
}

.footer-left {
  display: flex;
  align-items: center;
  gap: 8px;
  color: #53697f;
  font-family: var(--font-mono);
  font-size: 8.5px;
}

.footer-left .divider {
  width: 2px;
  height: 2px;
  border-radius: 50%;
  background: #34485c;
}

.footer-left .muted {
  color: #3f5468;
}

.footer-actions {
  display: flex;
  align-items: center;
  gap: 5px;
}

.footer-btn {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  height: 25px;
  padding: 0 8px;
  border: 1px solid transparent;
  border-radius: 6px;
  background: transparent;
  color: #62798f;
  font-family: inherit;
  font-size: 8px;
  cursor: pointer;
  transition:
    color 0.15s ease,
    background 0.15s ease;
}

.footer-btn:hover {
  color: #b6cedf;
  background: rgba(255, 255, 255, 0.035);
}

.footer-btn.danger:hover {
  color: #ffaaa4;
  background: rgba(255, 69, 58, 0.06);
}

.footer-btn.armed {
  color: #ffb2ac;
  background: rgba(255, 69, 58, 0.1);
}

.app-toast {
  position: fixed;
  left: 50%;
  bottom: 24px;
  z-index: 30;
  display: flex;
  align-items: center;
  gap: 8px;
  max-width: min(560px, calc(100vw - 40px));
  padding: 9px 13px;
  border: 1px solid rgba(148, 178, 209, 0.16);
  border-radius: 9px;
  background: rgba(8, 14, 23, 0.95);
  color: #cfe1f0;
  font-size: 11px;
  box-shadow: 0 16px 40px rgba(0, 0, 0, 0.45);
  backdrop-filter: blur(14px);
  transform: translateX(-50%);
}

.app-toast.ok {
  border-color: rgba(50, 215, 75, 0.2);
  color: #b6f0c4;
}

.app-toast.error {
  border-color: rgba(255, 69, 58, 0.24);
  color: #ffc1bc;
}

.toast-enter-active,
.toast-leave-active {
  transition:
    opacity 0.18s ease,
    transform 0.18s ease;
}

.toast-enter-from,
.toast-leave-to {
  opacity: 0;
  transform: translate(-50%, 6px);
}

@media (max-width: 920px) {
  .hero {
    grid-template-columns: minmax(0, 1fr) 320px;
    padding: 30px;
  }

  .dashboard-grid {
    grid-template-columns: 1fr;
  }

  .column {
    gap: 14px;
  }

  .recent-card,
  .examples-card {
    min-height: unset;
  }

  .capture-list {
    max-height: 220px;
  }
}

@media (max-width: 820px) {
  .app-header {
    padding: 0 18px;
  }

  .content {
    padding: 18px;
  }

  .hero {
    grid-template-columns: 1fr;
    min-height: unset;
    padding: 26px;
  }

  .hero-visual {
    display: none;
  }

  .status-pill,
  .preview-chip {
    display: none;
  }
}

@media (max-width: 580px) {
  .brand-sub {
    display: none;
  }

  .hero h1 {
    font-size: 29px;
  }

  .shortcut-display {
    display: none;
  }

  .setting-row {
    flex-direction: column;
    align-items: flex-start;
  }

  .setting-control {
    width: 100%;
    justify-content: flex-start;
  }

  .text-field {
    flex: 1;
    min-width: 0;
    width: auto;
  }

  .footer-left {
    flex-wrap: wrap;
  }
}
</style>
