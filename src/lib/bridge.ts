import { invoke } from "@tauri-apps/api/core";
import type {
  Annotation,
  AppStatus,
  CaptureEntry,
  ExportAction,
  ExportResult,
  OverlayBootstrap,
  OverlayFrame,
} from "./types";
import {
  mockCaptures,
  mockOverlayBootstrap,
  mockStatus,
  setMockAutoStart,
  setMockSaveDir,
  setMockShortcut,
} from "./mock";

const windowWithTauri = window as Window & { __TAURI_INTERNALS__?: unknown };
export const isTauri = Boolean(windowWithTauri.__TAURI_INTERNALS__);

function delay(ms: number): Promise<void> {
  return new Promise((resolve) => window.setTimeout(resolve, ms));
}

export async function loadOverlay(): Promise<OverlayFrame> {
  const frame = isTauri
    ? await invoke<OverlayBootstrap>("overlay_bootstrap", {})
    : await (async () => {
        await delay(140);
        return mockOverlayBootstrap();
      })();
  return { ...frame, imageUrl: frame.imageDataUrl };
}

export function loadImage(src: string): Promise<HTMLImageElement> {
  return new Promise((resolve, reject) => {
    const img = new Image();
    img.onload = () => resolve(img);
    img.onerror = () => reject(new Error("无法加载截图图像"));
    img.src = src;
  });
}

export async function overlayReady(): Promise<void> {
  if (!isTauri) return;
  await invoke("overlay_ready");
}

export async function cancelOverlay(): Promise<void> {
  if (!isTauri) {
    await delay(100);
    return;
  }
  await invoke("overlay_cancel");
}

export async function closeOverlay(captureId: string): Promise<void> {
  if (!isTauri) {
    await delay(120);
    return;
  }
  await invoke("overlay_close", { captureId });
}

export async function exportOverlay(
  captureId: string,
  pngDataUrl: string,
  action: ExportAction,
  annotations: Annotation[],
  region?: { x: number; y: number; width: number; height: number },
): Promise<ExportResult> {
  if (!isTauri) {
    await delay(420);
    return {
      ok: true,
      action,
      saved: action !== "clipboard",
      copied: action === "clipboard",
      canceled: false,
      path:
        action === "clipboard"
          ? null
          : "C:\\Users\\Demo\\Pictures\\SnapMCP\\SnapMCP_demo.png",
    };
  }
  return invoke<ExportResult>("export_capture", {
    captureId,
    pngDataUrl,
    action,
    annotations,
    region,
  });
}

export async function getStatus(): Promise<AppStatus> {
  if (!isTauri) {
    await delay(220);
    return mockStatus();
  }
  return invoke<AppStatus>("get_status");
}

export async function updateShortcut(shortcut: string): Promise<{
  shortcut: string;
  label: string;
}> {
  if (!isTauri) {
    await delay(280);
    setMockShortcut(shortcut);
    return { shortcut, label: shortcutLabel(shortcut) };
  }
  return invoke("update_shortcut", { shortcut });
}

export async function setAutoStart(enabled: boolean): Promise<boolean> {
  if (!isTauri) {
    await delay(260);
    setMockAutoStart(enabled);
    return enabled;
  }
  return invoke<boolean>("set_autostart", { enabled });
}

export async function chooseSaveDir(): Promise<string | null> {
  if (!isTauri) {
    await delay(320);
    const next = "D:\\Screenshots\\SnapMCP";
    setMockSaveDir(next);
    return next;
  }
  return invoke<string | null>("choose_save_dir");
}

export async function openSaveDir(): Promise<void> {
  if (!isTauri) return;
  await invoke("open_save_dir");
}

export async function openConfigDir(): Promise<void> {
  if (!isTauri) return;
  await invoke("open_config_dir");
}

export async function revealPath(path: string): Promise<void> {
  if (!isTauri) return;
  await invoke("reveal_path", { path });
}

export async function copyText(text: string): Promise<void> {
  if (!isTauri) {
    await navigator.clipboard?.writeText(text);
    return;
  }
  await invoke("copy_text", { text });
}

export async function captureNow(): Promise<void> {
  if (!isTauri) {
    const win = window.open(
      "/overlay.html?mock=1",
      "snapmcp-overlay-demo",
      "width=1440,height=900",
    );
    if (!win) {
      window.alert("浏览器阻止了弹窗，请直接访问 /overlay.html?mock=1 预览截图界面。");
    }
    return;
  }
  await invoke("capture_now");
}

export async function quitApp(): Promise<void> {
  if (!isTauri) return;
  await invoke("quit_app");
}

export async function listCaptures(limit = 8): Promise<CaptureEntry[]> {
  if (!isTauri) {
    await delay(200);
    return mockCaptures().slice(0, limit);
  }
  return invoke<CaptureEntry[]>("list_captures", { limit });
}

export function shortcutLabel(shortcut: string): string {
  const mac = /Mac|iPhone|iPad/.test(navigator.platform);
  return shortcut
    .split("+")
    .map((part) => {
      const key = part.trim();
      switch (key.toLowerCase()) {
        case "cmdorctrl":
        case "commandorcontrol":
        case "cmdorcontrol":
          return mac ? "⌘" : "Ctrl";
        case "ctrl":
        case "control":
          return mac ? "⌃" : "Ctrl";
        case "cmd":
        case "command":
        case "super":
        case "meta":
          return mac ? "⌘" : "Win";
        case "shift":
          return mac ? "⇧" : "Shift";
        case "alt":
        case "option":
          return mac ? "⌥" : "Alt";
        default:
          return key.toUpperCase();
      }
    })
    .join(mac ? "" : " + ");
}

export function isMac(): boolean {
  return /Mac|iPhone|iPad/.test(navigator.platform);
}
