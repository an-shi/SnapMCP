<script setup lang="ts">
import {
  computed,
  nextTick,
  onBeforeUnmount,
  onMounted,
  reactive,
  ref,
  shallowRef,
} from "vue";
import Icon from "../components/Icon.vue";
import Toolbar from "../components/Toolbar.vue";
import {
  cancelOverlay,
  closeOverlay,
  exportOverlay,
  isTauri,
  loadImage,
  loadOverlay,
  overlayReady,
} from "../lib/bridge";
import { withAlpha } from "../lib/colors";
import { AnnotationHistory } from "../lib/history";
import { draftAnnotation, drawAnnotations, renderComposite } from "../lib/renderer";
import type {
  Annotation,
  ExportAction,
  OverlayFrame,
  Point,
  Region,
  Tool,
  ToolStyle,
} from "../lib/types";

type Phase = "loading" | "select" | "edit" | "done" | "closed" | "error";

const phase = ref<Phase>("loading");
const frame = ref<OverlayFrame | null>(null);
const errorMessage = ref("");
const doneTitle = ref("");
const doneDetail = ref("");
const isMock = !isTauri;

const stageOverlay = ref<HTMLCanvasElement | null>(null);
const baseCanvas = ref<HTMLCanvasElement | null>(null);
const annCanvas = ref<HTMLCanvasElement | null>(null);
const textAreaRef = ref<HTMLTextAreaElement | null>(null);

const selection = ref<Region>({ x: 0, y: 0, width: 0, height: 0 });
const annotations = ref<Annotation[]>([]);
const canUndo = ref(false);
const canRedo = ref(false);
const draft = shallowRef<Annotation | null>(null);
const busy = ref(false);
const toast = ref<{ kind: "ok" | "error" | "info"; text: string } | null>(null);
const textAnchor = ref<{ x: number; y: number } | null>(null);
const textValue = ref("");
const cursorPoint = ref<{ x: number; y: number } | null>(null);

const style = reactive<ToolStyle>({
  tool: "arrow",
  color: "#FF453A",
  strokeWidth: 4,
  fontSize: 22,
  textBackground: true,
});

const viewport = reactive({ width: window.innerWidth, height: window.innerHeight });
const pxScale = ref(1);
const fit = ref(1);
const history = new AnnotationHistory();
let sourceImage: HTMLImageElement | null = null;
let dragMode: "select" | "draw" | null = null;
let dragStart: Point = [0, 0];
let ignoreTextBlur = false;
let toastTimer: number | undefined;
let closeTimer: number | undefined;

const shellStyle = computed(() => ({
  width: `${Math.max(1, selection.value.width * fit.value)}px`,
  height: `${Math.max(1, selection.value.height * fit.value)}px`,
}));

const sizeTagStyle = computed(() => {
  if (!frame.value) return {};
  const rx = viewport.width / frame.value.width;
  const ry = viewport.height / frame.value.height;
  const sel = selection.value;
  const centerX = (sel.x + sel.width / 2) * rx;
  const top = sel.y * ry - 30;
  return {
    left: `${centerX}px`,
    top: `${top < 8 ? (sel.y + sel.height) * ry + 10 : top}px`,
    transform: "translateX(-50%)",
  };
});

const textEditorStyle = computed(() => {
  if (!textAnchor.value || !frame.value) return {};
  const left = textAnchor.value.x * fit.value;
  const top = textAnchor.value.y * fit.value;
  const fontSize = Math.max(10, style.fontSize * pxScale.value * fit.value);
  const padding = Math.max(4, 10 * pxScale.value * fit.value);
  return {
    left: `${left}px`,
    top: `${top}px`,
    width: `${Math.max(150, Math.min(420, selection.value.width * fit.value - left - 8))}px`,
    color: style.color,
    fontSize: `${fontSize}px`,
    lineHeight: "1.35",
    padding: `${padding}px`,
    background:
      style.textBackground ? withAlpha("#0B1220", 0.88) : "rgba(8, 14, 24, 0.35)",
    borderColor: withAlpha(style.color, 0.85),
  } as Record<string, string>;
});

const cursorClass = computed(() => {
  if (style.tool === "text") return "cursor-text";
  if (style.tool === "step") return "cursor-copy";
  return "cursor-crosshair";
});

const rootClass = computed(() => ({
  "is-mock": isMock,
  "is-interactive": frame.value?.mode === "interactive",
  [`phase-${phase.value}`]: true,
}));

function errorText(error: unknown): string {
  if (typeof error === "string") return error;
  if (error instanceof Error) return error.message;
  return JSON.stringify(error);
}

function showToast(kind: "ok" | "error" | "info", text: string, timeout = 3600) {
  if (toastTimer) window.clearTimeout(toastTimer);
  toast.value = { kind, text };
  toastTimer = window.setTimeout(() => {
    toast.value = null;
  }, timeout);
}

function syncHistory() {
  canUndo.value = history.canUndo;
  canRedo.value = history.canRedo;
}

function commit(next: Annotation[]) {
  annotations.value = history.push(next);
  syncHistory();
  redrawAnnotations();
}

function syncScale() {
  if (!frame.value) return;
  viewport.width = window.innerWidth;
  viewport.height = window.innerHeight;
  pxScale.value = Math.max(0.1, frame.value.width / Math.max(1, window.innerWidth));
}

async function onMountedInit() {
  window.addEventListener("keydown", onKeyDown, true);
  window.addEventListener("pointermove", onPointerMove, { passive: false });
  window.addEventListener("pointerup", onPointerUp);
  window.addEventListener("pointercancel", onPointerUp);
  window.addEventListener("resize", onResize);
  try {
    const loaded = await loadOverlay();
    frame.value = loaded;
    sourceImage = await loadImage(loaded.imageUrl);
    syncScale();
    phase.value = "select";
    await nextTick();
    sizeStageOverlay();
    drawSelectionOverlay();
    await nextTick();
    await new Promise<void>((resolve) =>
      requestAnimationFrame(() => requestAnimationFrame(() => resolve())),
    );
    await overlayReady();
  } catch (error) {
    errorMessage.value = errorText(error);
    phase.value = "error";
    await overlayReady().catch(() => undefined);
  }
}

onMounted(() => {
  void onMountedInit();
});

onBeforeUnmount(() => {
  window.removeEventListener("keydown", onKeyDown, true);
  window.removeEventListener("pointermove", onPointerMove);
  window.removeEventListener("pointerup", onPointerUp);
  window.removeEventListener("pointercancel", onPointerUp);
  window.removeEventListener("resize", onResize);
  if (toastTimer) window.clearTimeout(toastTimer);
  if (closeTimer) window.clearTimeout(closeTimer);
});

function sizeStageOverlay() {
  const canvas = stageOverlay.value;
  if (!canvas || !frame.value) return;
  const dpr = Math.min(window.devicePixelRatio || 1, 2);
  canvas.width = Math.max(1, Math.round(window.innerWidth * dpr));
  canvas.height = Math.max(1, Math.round(window.innerHeight * dpr));
  canvas.style.width = `${window.innerWidth}px`;
  canvas.style.height = `${window.innerHeight}px`;
}

function drawSelectionOverlay() {
  const canvas = stageOverlay.value;
  if (!canvas || !frame.value) return;
  const ctx = canvas.getContext("2d");
  if (!ctx) return;
  const sx = canvas.width / frame.value.width;
  const sy = canvas.height / frame.value.height;
  ctx.setTransform(sx, 0, 0, sy, 0, 0);
  ctx.clearRect(0, 0, frame.value.width, frame.value.height);
  ctx.fillStyle = "rgba(3, 6, 12, 0.58)";
  ctx.fillRect(0, 0, frame.value.width, frame.value.height);
  const sel = normalizedSelection();
  if (sel.width > 0 && sel.height > 0) {
    ctx.clearRect(sel.x, sel.y, sel.width, sel.height);
    ctx.save();
    ctx.strokeStyle = "#67E8F9";
    ctx.lineWidth = Math.max(1, 2 * pxScale.value);
    ctx.strokeRect(
      sel.x + ctx.lineWidth / 2,
      sel.y + ctx.lineWidth / 2,
      Math.max(1, sel.width - ctx.lineWidth),
      Math.max(1, sel.height - ctx.lineWidth),
    );
    const corner = Math.max(8, 14 * pxScale.value);
    ctx.strokeStyle = "#FFFFFF";
    ctx.lineWidth = Math.max(1.5, 2.5 * pxScale.value);
    const corners = [
      [sel.x, sel.y, 1, 1],
      [sel.x + sel.width, sel.y, -1, 1],
      [sel.x, sel.y + sel.height, 1, -1],
      [sel.x + sel.width, sel.y + sel.height, -1, -1],
    ];
    for (const [x, y, dx, dy] of corners) {
      ctx.beginPath();
      ctx.moveTo(x + dx * corner, y);
      ctx.lineTo(x, y);
      ctx.lineTo(x, y + dy * corner);
      ctx.stroke();
    }
    ctx.restore();
  }
}

function normalizedSelection(): Region {
  const s = selection.value;
  return {
    x: Math.min(s.x, s.x + s.width),
    y: Math.min(s.y, s.y + s.height),
    width: Math.abs(s.width),
    height: Math.abs(s.height),
  };
}

function stagePoint(event: PointerEvent | MouseEvent): Point {
  const canvas = stageOverlay.value;
  if (!canvas || !frame.value) return [0, 0];
  const rect = canvas.getBoundingClientRect();
  const x = ((event.clientX - rect.left) / Math.max(1, rect.width)) * frame.value.width;
  const y = ((event.clientY - rect.top) / Math.max(1, rect.height)) * frame.value.height;
  return [
    Math.max(0, Math.min(frame.value.width, x)),
    Math.max(0, Math.min(frame.value.height, y)),
  ];
}

function canvasPoint(event: PointerEvent | MouseEvent): Point {
  const canvas = annCanvas.value;
  const sel = normalizedSelection();
  if (!canvas) return [0, 0];
  const rect = canvas.getBoundingClientRect();
  const x = ((event.clientX - rect.left) / Math.max(1, rect.width)) * sel.width;
  const y = ((event.clientY - rect.top) / Math.max(1, rect.height)) * sel.height;
  return [
    Math.max(0, Math.min(sel.width, x)),
    Math.max(0, Math.min(sel.height, y)),
  ];
}

function onStagePointerDown(event: PointerEvent) {
  if (event.button !== 0 || phase.value !== "select") return;
  event.preventDefault();
  const [x, y] = stagePoint(event);
  dragStart = [x, y];
  dragMode = "select";
  selection.value = { x, y, width: 0, height: 0 };
  drawSelectionOverlay();
}

function onStageHover(event: PointerEvent) {
  if (phase.value !== "select") return;
  const [x, y] = stagePoint(event);
  cursorPoint.value = { x: Math.round(x), y: Math.round(y) };
}

function onPointerMove(event: PointerEvent) {
  if (dragMode === "select" && phase.value === "select") {
    event.preventDefault();
    const [x, y] = stagePoint(event);
    let dx = x - dragStart[0];
    let dy = y - dragStart[1];
    if (event.shiftKey) {
      const size = Math.max(Math.abs(dx), Math.abs(dy));
      dx = Math.sign(dx || 1) * size;
      dy = Math.sign(dy || 1) * size;
    }
    const endX = Math.max(
      0,
      Math.min(frame.value!.width, dragStart[0] + dx),
    );
    const endY = Math.max(
      0,
      Math.min(frame.value!.height, dragStart[1] + dy),
    );
    selection.value = {
      x: Math.min(dragStart[0], endX),
      y: Math.min(dragStart[1], endY),
      width: Math.abs(endX - dragStart[0]),
      height: Math.abs(endY - dragStart[1]),
    };
    drawSelectionOverlay();
    return;
  }

  if (dragMode === "draw" && phase.value === "edit") {
    event.preventDefault();
    const point = canvasPoint(event);
    updateDraft(point, event.shiftKey);
  }
}

function onPointerUp() {
  if (dragMode === "select") {
    const sel = normalizedSelection();
    dragMode = null;
    const min = Math.max(3, 4 * pxScale.value);
    if (sel.width >= min && sel.height >= min) {
      void enterEdit(sel);
    } else {
      selection.value = { x: 0, y: 0, width: 0, height: 0 };
      drawSelectionOverlay();
    }
    return;
  }
  if (dragMode === "draw") {
    dragMode = null;
    commitDraft();
  }
}

async function enterEdit(raw: Region) {
  if (!frame.value) return;
  const x = Math.max(0, Math.round(raw.x));
  const y = Math.max(0, Math.round(raw.y));
  const width = Math.max(1, Math.min(Math.round(raw.width), frame.value.width - x));
  const height = Math.max(1, Math.min(Math.round(raw.height), frame.value.height - y));
  selection.value = { x, y, width, height };
  history.reset();
  annotations.value = [];
  syncHistory();
  phase.value = "edit";
  sourceImage = sourceImage ?? (await loadImage(frame.value.imageUrl));
  await nextTick();
  prepareCrop();
  layoutEditor();
}

function prepareCrop() {
  const canvas = baseCanvas.value;
  if (!canvas || !sourceImage) return;
  const sel = selection.value;
  canvas.width = sel.width;
  canvas.height = sel.height;
  const ctx = canvas.getContext("2d");
  if (!ctx) return;
  ctx.clearRect(0, 0, sel.width, sel.height);
  ctx.drawImage(
    sourceImage,
    sel.x,
    sel.y,
    sel.width,
    sel.height,
    0,
    0,
    sel.width,
    sel.height,
  );
}

function layoutEditor() {
  if (!frame.value || phase.value !== "edit") return;
  const sel = selection.value;
  viewport.width = window.innerWidth;
  viewport.height = window.innerHeight;
  const availW = Math.max(160, window.innerWidth - 96);
  const availH = Math.max(140, window.innerHeight - 270);
  fit.value = Math.min(1, availW / sel.width, availH / sel.height);
  if (!Number.isFinite(fit.value) || fit.value <= 0) fit.value = 1;
  const canvas = annCanvas.value;
  if (!canvas) return;
  const dpr = Math.min(window.devicePixelRatio || 1, 2);
  const cssW = Math.max(1, sel.width * fit.value);
  const cssH = Math.max(1, sel.height * fit.value);
  canvas.width = Math.max(1, Math.round(cssW * dpr));
  canvas.height = Math.max(1, Math.round(cssH * dpr));
  canvas.style.width = `${cssW}px`;
  canvas.style.height = `${cssH}px`;
  redrawAnnotations();
}

function redrawAnnotations() {
  const canvas = annCanvas.value;
  if (!canvas) return;
  const ctx = canvas.getContext("2d");
  if (!ctx) return;
  ctx.setTransform(1, 0, 0, 1, 0, 0);
  ctx.clearRect(0, 0, canvas.width, canvas.height);
  const sel = selection.value;
  if (sel.width <= 0 || sel.height <= 0) return;
  const scale = canvas.width / sel.width;
  ctx.setTransform(scale, 0, 0, scale, 0, 0);
  drawAnnotations(ctx, annotations.value, baseCanvas.value ?? undefined);
  if (draft.value) {
    if (draft.value.type === "mosaic") {
      ctx.save();
      ctx.setLineDash([6 * pxScale.value, 4 * pxScale.value]);
      ctx.strokeStyle = "#67E8F9";
      ctx.lineWidth = Math.max(1, 1.5 * pxScale.value);
      ctx.fillStyle = "rgba(34,211,238,0.08)";
      ctx.fillRect(draft.value.x, draft.value.y, draft.value.w, draft.value.h);
      ctx.strokeRect(draft.value.x, draft.value.y, draft.value.w, draft.value.h);
      ctx.restore();
    } else {
      draftAnnotation(draft.value, ctx, baseCanvas.value ?? undefined, pxScale.value);
    }
  }
}

function strokeImagePx(): number {
  return Math.max(1, Number((style.strokeWidth * pxScale.value).toFixed(2)));
}

function constrainEnd(start: Point, end: Point, shift: boolean): Point {
  if (!shift) return end;
  const dx = end[0] - start[0];
  const dy = end[1] - start[1];
  if (style.tool === "arrow" || style.tool === "line" || style.tool === "pen") {
    const angle = Math.atan2(dy, dx);
    const step = Math.PI / 4;
    const snapped = Math.round(angle / step) * step;
    const length = Math.hypot(dx, dy);
    return [start[0] + Math.cos(snapped) * length, start[1] + Math.sin(snapped) * length];
  }
  const size = Math.max(Math.abs(dx), Math.abs(dy));
  return [
    start[0] + Math.sign(dx || 1) * size,
    start[1] + Math.sign(dy || 1) * size,
  ];
}

function buildShape(start: Point, rawEnd: Point, shift: boolean): Annotation | null {
  const end = constrainEnd(start, rawEnd, shift);
  const stroke = strokeImagePx();
  const x = Math.min(start[0], end[0]);
  const y = Math.min(start[1], end[1]);
  const w = Math.abs(end[0] - start[0]);
  const h = Math.abs(end[1] - start[1]);
  switch (style.tool) {
    case "rect":
      return { type: "rect", x, y, w, h, color: style.color, strokeWidth: stroke };
    case "ellipse":
      return { type: "ellipse", x, y, w, h, color: style.color, strokeWidth: stroke };
    case "arrow":
      return {
        type: "arrow",
        from: start,
        to: end,
        color: style.color,
        strokeWidth: stroke,
      };
    case "line":
      return {
        type: "line",
        from: start,
        to: end,
        color: style.color,
        strokeWidth: stroke,
      };
    case "highlight":
      return {
        type: "highlight",
        x,
        y,
        w,
        h,
        color: withAlpha(style.color, 0.3),
      };
    case "mosaic":
      return {
        type: "mosaic",
        x,
        y,
        w,
        h,
        blockSize: Math.max(6, Math.round(16 * pxScale.value)),
      };
    default:
      return null;
  }
}

function onCanvasPointerDown(event: PointerEvent) {
  if (event.button !== 0 || phase.value !== "edit" || busy.value) return;
  event.preventDefault();
  if (textAnchor.value) {
    commitText();
    return;
  }
  const point = canvasPoint(event);
  if (style.tool === "text") {
    openText(point);
    return;
  }
  if (style.tool === "step") {
    addStep(point);
    return;
  }
  dragStart = point;
  dragMode = "draw";
  if (style.tool === "pen") {
    draft.value = {
      type: "pen",
      points: [point],
      color: style.color,
      strokeWidth: strokeImagePx(),
    };
  } else {
    draft.value = buildShape(point, point, event.shiftKey);
  }
  redrawAnnotations();
}

function updateDraft(point: Point, shift: boolean) {
  if (style.tool === "pen") {
    const current = draft.value;
    if (!current || current.type !== "pen") return;
    const last = current.points[current.points.length - 1];
    if (!last) return;
    const distance = Math.hypot(point[0] - last[0], point[1] - last[1]);
    if (distance < Math.max(1.2, pxScale.value)) return;
    if (current.points.length >= 5000) return;
    draft.value = { ...current, points: [...current.points, point] };
  } else {
    draft.value = buildShape(dragStart, point, shift);
  }
  redrawAnnotations();
}

function commitDraft() {
  const shape = draft.value;
  draft.value = null;
  if (!shape) {
    redrawAnnotations();
    return;
  }
  let valid = true;
  if (
    shape.type === "rect" ||
    shape.type === "ellipse" ||
    shape.type === "highlight" ||
    shape.type === "mosaic"
  ) {
    valid = shape.w >= 2 && shape.h >= 2;
  } else if (shape.type === "arrow" || shape.type === "line") {
    valid = Math.hypot(shape.to[0] - shape.from[0], shape.to[1] - shape.from[1]) >= 4;
  } else if (shape.type === "pen") {
    valid = shape.points.length >= 2;
  }
  if (!valid) {
    redrawAnnotations();
    return;
  }
  commit([...annotations.value, shape]);
}

function openText(point: Point) {
  const sel = selection.value;
  const x = Math.min(point[0], Math.max(0, sel.width - 60 * pxScale.value));
  const y = Math.min(point[1], Math.max(0, sel.height - 30 * pxScale.value));
  textAnchor.value = { x, y };
  textValue.value = "";
  ignoreTextBlur = false;
  nextTick(() => {
    const area = textAreaRef.value;
    if (!area) return;
    area.focus();
    resizeTextArea();
  });
}

function commitText() {
  const anchor = textAnchor.value;
  const value = textValue.value.trim();
  textAnchor.value = null;
  textValue.value = "";
  ignoreTextBlur = true;
  nextTick(() => {
    ignoreTextBlur = false;
  });
  if (!anchor || !value) {
    redrawAnnotations();
    return;
  }
  const text: Annotation = {
    type: "text",
    x: anchor.x,
    y: anchor.y,
    text: value,
    color: style.color,
    fontSize: Math.max(10, Math.round(style.fontSize * pxScale.value)),
    bgColor: style.textBackground ? withAlpha("#0B1220", 0.86) : "none",
    padding: Math.max(3, Math.round(10 * pxScale.value)),
  };
  commit([...annotations.value, text]);
}

function cancelText() {
  ignoreTextBlur = true;
  textAnchor.value = null;
  textValue.value = "";
  nextTick(() => {
    ignoreTextBlur = false;
    redrawAnnotations();
  });
}

function onTextBlur() {
  if (ignoreTextBlur) return;
  commitText();
}

function resizeTextArea() {
  const area = textAreaRef.value;
  if (!area) return;
  area.style.height = "auto";
  area.style.height = `${Math.max(36, area.scrollHeight)}px`;
}

function addStep(point: Point) {
  let max = 0;
  for (const ann of annotations.value) {
    if (ann.type === "step") max = Math.max(max, ann.index);
  }
  const step: Annotation = {
    type: "step",
    x: point[0],
    y: point[1],
    index: max + 1,
    color: style.color,
    size: Math.max(18, Math.round(30 * pxScale.value)),
  };
  commit([...annotations.value, step]);
}

function handleUndo() {
  const next = history.undo();
  if (!next) return;
  annotations.value = next;
  syncHistory();
  redrawAnnotations();
}

function handleRedo() {
  const next = history.redo();
  if (!next) return;
  annotations.value = next;
  syncHistory();
  redrawAnnotations();
}

function setTool(tool: Tool) {
  if (textAnchor.value) commitText();
  style.tool = tool;
}

function reselect() {
  if (busy.value) return;
  if (textAnchor.value) cancelText();
  annotations.value = [];
  history.reset();
  syncHistory();
  draft.value = null;
  selection.value = { x: 0, y: 0, width: 0, height: 0 };
  phase.value = "select";
  nextTick(() => {
    sizeStageOverlay();
    drawSelectionOverlay();
  });
}

async function doExport(action: ExportAction) {
  if (!frame.value || !baseCanvas.value || busy.value) return;
  busy.value = true;
  try {
    await new Promise<void>((resolve) => requestAnimationFrame(() => resolve()));
    const composite = renderComposite(baseCanvas.value, annotations.value);
    const dataUrl = composite.toDataURL("image/png");
    const scaleX = frame.value.display.width / frame.value.width;
    const scaleY = frame.value.display.height / frame.value.height;
    const region = {
      x: Math.round(frame.value.display.x + selection.value.x * scaleX),
      y: Math.round(frame.value.display.y + selection.value.y * scaleY),
      width: Math.max(1, Math.round(selection.value.width * scaleX)),
      height: Math.max(1, Math.round(selection.value.height * scaleY)),
    };
    const result = await exportOverlay(
      frame.value.captureId,
      dataUrl,
      action,
      annotations.value,
      region,
    );
    if (result.canceled) {
      busy.value = false;
      showToast("info", "已取消保存");
      return;
    }
    if (action === "clipboard") {
      finish("已复制到剪贴板", "现在可以直接粘贴到聊天窗口或文档中。");
    } else if (action === "confirm") {
      finish("已发送给 AI", "AI 已收到这张标注截图，可以继续对话。");
    } else {
      finish("已保存标注截图", result.path ?? "PNG 已写入截图文件夹。");
    }
  } catch (error) {
    busy.value = false;
    showToast("error", errorText(error), 5000);
  }
}

function finish(title: string, detail: string) {
  doneTitle.value = title;
  doneDetail.value = detail;
  phase.value = "done";
  busy.value = true;
  if (!isTauri) {
    busy.value = false;
    return;
  }
  closeTimer = window.setTimeout(async () => {
    try {
      if (frame.value) await closeOverlay(frame.value.captureId);
    } catch {
      busy.value = false;
    }
  }, 720);
}

async function cancelCapture() {
  if (busy.value && phase.value !== "done") return;
  busy.value = true;
  try {
    await cancelOverlay();
    if (!isTauri) {
      phase.value = "closed";
      busy.value = false;
    }
  } catch (error) {
    busy.value = false;
    showToast("error", errorText(error), 5000);
  }
}

async function closeAfterDone() {
  if (!frame.value) return;
  try {
    await closeOverlay(frame.value.captureId);
  } catch (error) {
    showToast("error", errorText(error));
  }
}

function reloadMock() {
  window.location.reload();
}

function isTypingTarget(target: EventTarget | null): boolean {
  const el = target as HTMLElement | null;
  if (!el) return false;
  const tag = el.tagName?.toLowerCase();
  return tag === "input" || tag === "textarea" || el.isContentEditable;
}

function onKeyDown(event: KeyboardEvent) {
  if (phase.value === "loading") return;

  if (textAnchor.value) {
    if (event.key === "Escape") {
      event.preventDefault();
      event.stopPropagation();
      cancelText();
    }
    return;
  }

  if (event.key === "Escape") {
    event.preventDefault();
    event.stopPropagation();
    if (phase.value === "done") {
      void closeAfterDone();
      return;
    }
    if (phase.value === "error" || phase.value === "closed") {
      if (!isTauri) reloadMock();
      else void cancelCapture();
      return;
    }
    if (phase.value === "select" || phase.value === "edit") {
      void cancelCapture();
    }
    return;
  }

  if (phase.value !== "edit" || isTypingTarget(event.target)) return;
  const meta = event.ctrlKey || event.metaKey;
  const key = event.key.toLowerCase();

  if (meta) {
    if (key === "z") {
      event.preventDefault();
      if (event.shiftKey) handleRedo();
      else handleUndo();
      return;
    }
    if (key === "y") {
      event.preventDefault();
      handleRedo();
      return;
    }
    if (key === "c" && frame.value?.mode === "capture") {
      event.preventDefault();
      void doExport("clipboard");
      return;
    }
    if (key === "s") {
      event.preventDefault();
      if (frame.value?.mode === "capture") {
        void doExport(event.shiftKey ? "saveAs" : "save");
      }
      return;
    }
    if (key === "enter" && frame.value?.mode === "interactive") {
      event.preventDefault();
      void doExport("confirm");
      return;
    }
    return;
  }

  if (event.key === "Enter") {
    if ((document.activeElement?.tagName ?? "").toLowerCase() === "button") return;
    if (frame.value?.mode === "interactive") {
      event.preventDefault();
      void doExport("confirm");
      return;
    }
    if (frame.value?.mode === "capture") {
      event.preventDefault();
      void doExport("clipboard");
      return;
    }
  }

  const toolMap: Record<string, Tool> = {
    r: "rect",
    e: "ellipse",
    a: "arrow",
    l: "line",
    p: "pen",
    t: "text",
    h: "highlight",
    m: "mosaic",
    s: "step",
  };
  if (toolMap[key]) {
    event.preventDefault();
    setTool(toolMap[key]);
  }
}

function onResize() {
  viewport.width = window.innerWidth;
  viewport.height = window.innerHeight;
  syncScale();
  if (phase.value === "select") {
    sizeStageOverlay();
    drawSelectionOverlay();
  } else if (phase.value === "edit") {
    layoutEditor();
  }
}

function reportError() {
  void cancelCapture();
}
</script>

<template>
  <div class="overlay-root" :class="rootClass" @contextmenu.prevent>
    <template v-if="phase === 'select' && frame">
      <img
        class="stage-image"
        :src="frame.imageUrl"
        alt=""
        draggable="false"
      />
      <canvas
        ref="stageOverlay"
        class="stage-overlay"
        @pointerdown="onStagePointerDown"
        @pointermove="onStageHover"
        @pointerleave="cursorPoint = null"
      />

      <div class="select-top">
        <div class="brand-chip">
          <span class="brand-mark"><Icon name="camera" :size="14" /></span>
          <span>SnapMCP</span>
        </div>
        <div class="select-hint">
          <Icon name="crosshair" :size="15" />
          <span>拖动框选要标注的区域</span>
          <kbd>Esc</kbd>
          <span class="muted">取消</span>
        </div>
      </div>

      <div v-if="frame.mode === 'interactive'" class="ai-banner">
        <span class="ai-pulse" />
        <span class="ai-label">AI 请求截图</span>
        <span v-if="frame.hint" class="ai-hint">{{ frame.hint }}</span>
      </div>

      <div class="select-footer">
        <span class="tip"><Icon name="mouse" :size="13" /> 按住 Shift 可框选正方形</span>
        <span v-if="cursorPoint" class="coord">
          X {{ cursorPoint.x }} · Y {{ cursorPoint.y }} px
        </span>
      </div>

      <div
        v-if="selection.width > 1 && selection.height > 1"
        class="size-tag"
        :style="sizeTagStyle"
      >
        {{ Math.round(selection.width) }} × {{ Math.round(selection.height) }}
      </div>

      <div v-if="isMock" class="preview-badge">浏览器预览</div>
    </template>

    <template v-else-if="phase === 'edit' && frame">
      <div class="editor-backdrop" />
      <div class="edit-topbar">
        <span class="brand-chip">
          <span class="brand-mark"><Icon name="pen" :size="13" /></span>
          <span>标注</span>
        </span>
        <span class="dim-chip">{{ selection.width }} × {{ selection.height }} px</span>
        <span class="topbar-spacer" />
        <span class="key-hints">
          <kbd>R</kbd> 矩形 <kbd>A</kbd> 箭头 <kbd>T</kbd> 文字 <kbd>⌘Z</kbd> 撤销
        </span>
        <span v-if="isMock" class="preview-badge inline">浏览器预览</span>
      </div>

      <div
        v-if="frame.mode === 'interactive'"
        class="ai-banner edit-ai"
      >
        <span class="ai-pulse" />
        <span class="ai-label">AI 正在等待</span>
        <span class="ai-hint">标注完成后点击“发送给 AI”</span>
      </div>

      <div class="editor-stage">
        <div class="crop-shell" :style="shellStyle">
          <canvas ref="baseCanvas" class="base-canvas" />
          <canvas
            ref="annCanvas"
            class="ann-canvas"
            :class="cursorClass"
            @pointerdown="onCanvasPointerDown"
          />
          <textarea
            v-if="textAnchor"
            ref="textAreaRef"
            v-model="textValue"
            class="text-input"
            :style="textEditorStyle"
            placeholder="输入文字…"
            spellcheck="false"
            @pointerdown.stop
            @input="resizeTextArea"
            @blur="onTextBlur"
            @keydown.enter.exact.prevent="commitText"
            @keydown.ctrl.enter.prevent="commitText"
            @keydown.meta.enter.prevent="commitText"
            @keydown.esc.stop.prevent="cancelText"
          />
        </div>
      </div>

      <Toolbar
        :tool="style.tool"
        :color="style.color"
        :stroke-width="style.strokeWidth"
        :font-size="style.fontSize"
        :text-background="style.textBackground"
        :can-undo="canUndo"
        :can-redo="canRedo"
        :interactive="frame.mode === 'interactive'"
        :busy="busy"
        :count="annotations.length"
        @update:tool="setTool"
        @update:color="style.color = $event"
        @update:strokeWidth="style.strokeWidth = $event"
        @update:fontSize="style.fontSize = $event"
        @update:textBackground="style.textBackground = $event"
        @undo="handleUndo"
        @redo="handleRedo"
        @copy="doExport('clipboard')"
        @save="doExport('save')"
        @saveAs="doExport('saveAs')"
        @confirm="doExport('confirm')"
        @cancel="cancelCapture"
        @reselect="reselect"
      />

      <div v-if="busy" class="working-chip">
        <span class="spinner" />
        正在处理…
      </div>

      <Transition name="toast">
        <div v-if="toast" class="toast" :class="toast.kind">
          <Icon
            :name="toast.kind === 'error' ? 'info' : toast.kind === 'ok' ? 'check' : 'dot'"
            :size="15"
          />
          <span>{{ toast.text }}</span>
        </div>
      </Transition>
    </template>

    <div v-else-if="phase === 'loading'" class="state-screen loading-screen">
      <div class="loading-card">
        <div class="brand-lockup">
          <span class="brand-mark large"><Icon name="camera" :size="22" /></span>
          <span class="brand-name">SnapMCP</span>
        </div>
        <span class="spinner large" />
        <p>正在准备截图与标注画布…</p>
      </div>
    </div>

    <div v-else class="state-screen">
      <div class="result-card" :class="{ error: phase === 'error' }">
        <div class="result-icon" :class="phase">
          <Icon
            :name="phase === 'error' ? 'info' : phase === 'closed' ? 'close' : 'check'"
            :size="26"
          />
        </div>
        <h1 v-if="phase === 'done'">{{ doneTitle }}</h1>
        <h1 v-else-if="phase === 'error'">截图初始化失败</h1>
        <h1 v-else>截图已取消</h1>
        <p v-if="phase === 'done'" class="result-detail">{{ doneDetail }}</p>
        <p v-else-if="phase === 'error'" class="result-detail error-text">
          {{ errorMessage }}
        </p>
        <p v-else class="result-detail">没有生成任何图片。</p>

        <div class="result-actions">
          <template v-if="phase === 'done' && isTauri">
            <button class="result-btn primary" type="button" @click="closeAfterDone">
              关闭窗口
            </button>
          </template>
          <template v-else-if="isMock">
            <button class="result-btn primary" type="button" @click="reloadMock">
              重新体验
            </button>
          </template>
          <template v-else>
            <button class="result-btn" type="button" @click="reportError">
              关闭
            </button>
          </template>
        </div>
        <span v-if="isMock" class="mock-note">浏览器预览模式 · 不会写入文件或剪贴板</span>
      </div>
    </div>
  </div>
</template>

<style scoped>
.overlay-root {
  position: fixed;
  inset: 0;
  overflow: hidden;
  color: #e9f1fb;
  font-family: var(--font-ui);
  background: #04070c;
  user-select: none;
  -webkit-app-region: no-drag;
}

.overlay-root * {
  -webkit-app-region: no-drag;
}

.stage-image,
.stage-overlay {
  position: absolute;
  inset: 0;
  width: 100%;
  height: 100%;
  display: block;
}

.stage-image {
  object-fit: fill;
  pointer-events: none;
  -webkit-user-drag: none;
}

.stage-overlay {
  cursor: crosshair;
  touch-action: none;
}

.select-top {
  position: absolute;
  top: 20px;
  left: 50%;
  transform: translateX(-50%);
  display: flex;
  align-items: center;
  gap: 12px;
  z-index: 10;
  pointer-events: none;
}

.brand-chip {
  display: inline-flex;
  align-items: center;
  gap: 7px;
  height: 34px;
  padding: 0 12px 0 6px;
  border: 1px solid rgba(151, 198, 227, 0.16);
  border-radius: 11px;
  background: rgba(7, 13, 22, 0.83);
  box-shadow: 0 10px 30px rgba(0, 0, 0, 0.28);
  color: #eaf7ff;
  font-size: 12px;
  font-weight: 700;
  letter-spacing: 0.01em;
  backdrop-filter: blur(14px);
}

.brand-mark {
  width: 24px;
  height: 24px;
  display: grid;
  place-items: center;
  border-radius: 7px;
  background: linear-gradient(145deg, #22d3ee, #0a84ff);
  color: #04141d;
  box-shadow: 0 4px 12px rgba(34, 211, 238, 0.22);
}

.brand-mark.large {
  width: 42px;
  height: 42px;
  border-radius: 12px;
}

.select-hint {
  display: inline-flex;
  align-items: center;
  gap: 8px;
  height: 34px;
  padding: 0 13px;
  border: 1px solid rgba(151, 198, 227, 0.13);
  border-radius: 11px;
  background: rgba(7, 13, 22, 0.78);
  color: #c3d7e9;
  font-size: 11px;
  backdrop-filter: blur(14px);
  box-shadow: 0 10px 30px rgba(0, 0, 0, 0.24);
}

.select-hint :deep(svg) {
  color: #67e8f9;
}

.select-hint kbd,
.key-hints kbd {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  min-width: 19px;
  height: 19px;
  padding: 0 5px;
  border: 1px solid rgba(255, 255, 255, 0.13);
  border-radius: 5px;
  background: rgba(255, 255, 255, 0.06);
  color: #dceaf7;
  font-family: var(--font-mono);
  font-size: 9px;
  line-height: 1;
}

.select-hint .muted {
  color: #7188a0;
}

.select-footer {
  position: absolute;
  left: 24px;
  bottom: 22px;
  display: flex;
  align-items: center;
  gap: 16px;
  color: #8ea3b8;
  font-size: 10px;
  z-index: 10;
  pointer-events: none;
}

.tip {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 7px 10px;
  border-radius: 8px;
  background: rgba(5, 10, 18, 0.74);
  border: 1px solid rgba(255, 255, 255, 0.06);
  backdrop-filter: blur(10px);
}

.coord {
  font-family: var(--font-mono);
  font-size: 10px;
  color: #9fb4c8;
  padding: 7px 9px;
  border-radius: 8px;
  background: rgba(5, 10, 18, 0.74);
  border: 1px solid rgba(255, 255, 255, 0.06);
}

.size-tag {
  position: absolute;
  z-index: 12;
  padding: 4px 8px;
  border: 1px solid rgba(103, 232, 249, 0.45);
  border-radius: 6px;
  background: rgba(5, 15, 24, 0.9);
  color: #b9f3ff;
  font-family: var(--font-mono);
  font-size: 10px;
  white-space: nowrap;
  pointer-events: none;
  box-shadow: 0 5px 18px rgba(0, 0, 0, 0.35);
}

.ai-banner {
  position: absolute;
  top: 72px;
  left: 50%;
  transform: translateX(-50%);
  z-index: 11;
  display: inline-flex;
  align-items: center;
  gap: 8px;
  max-width: min(620px, calc(100vw - 60px));
  padding: 8px 13px;
  border: 1px solid rgba(34, 211, 238, 0.24);
  border-radius: 99px;
  background: rgba(5, 17, 28, 0.86);
  box-shadow: 0 8px 28px rgba(0, 0, 0, 0.3);
  color: #b9eaf2;
  font-size: 11px;
  backdrop-filter: blur(14px);
  pointer-events: none;
}

.ai-pulse {
  width: 7px;
  height: 7px;
  flex: none;
  border-radius: 50%;
  background: #22d3ee;
  box-shadow: 0 0 0 4px rgba(34, 211, 238, 0.11);
  animation: pulse 1.8s ease-in-out infinite;
}

@keyframes pulse {
  0%,
  100% {
    box-shadow: 0 0 0 4px rgba(34, 211, 238, 0.09);
  }
  50% {
    box-shadow: 0 0 0 8px rgba(34, 211, 238, 0.02);
  }
}

.ai-label {
  font-weight: 700;
  color: #dffcff;
  white-space: nowrap;
}

.ai-hint {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  color: #8eb6c6;
}

.preview-badge {
  position: absolute;
  right: 18px;
  bottom: 18px;
  z-index: 13;
  padding: 5px 9px;
  border: 1px solid rgba(255, 255, 255, 0.1);
  border-radius: 7px;
  background: rgba(7, 13, 22, 0.8);
  color: #7f93a9;
  font-size: 9px;
  letter-spacing: 0.04em;
  backdrop-filter: blur(10px);
}

.preview-badge.inline {
  position: static;
  margin-left: 6px;
}

/* Editor */
.editor-backdrop {
  position: absolute;
  inset: 0;
  background:
    radial-gradient(ellipse 52% 42% at 50% 44%, rgba(30, 58, 83, 0.24), transparent 70%),
    radial-gradient(ellipse 70% 55% at 50% 104%, rgba(13, 31, 48, 0.38), transparent 72%),
    #05090f;
}

.editor-backdrop::before {
  content: "";
  position: absolute;
  inset: 0;
  opacity: 0.4;
  background-image: radial-gradient(rgba(129, 168, 199, 0.2) 0.7px, transparent 0.7px);
  background-size: 22px 22px;
  mask-image: radial-gradient(ellipse 62% 58% at 50% 48%, black 20%, transparent 76%);
}

.edit-topbar {
  position: absolute;
  top: 0;
  left: 0;
  right: 0;
  z-index: 30;
  height: 52px;
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 10px 18px;
  background: linear-gradient(180deg, rgba(4, 8, 14, 0.88), rgba(4, 8, 14, 0));
  font-size: 10px;
  pointer-events: none;
}

.dim-chip {
  padding: 4px 8px;
  border-radius: 6px;
  background: rgba(255, 255, 255, 0.045);
  color: #7089a2;
  font-family: var(--font-mono);
  font-size: 9px;
  white-space: nowrap;
}

.topbar-spacer {
  flex: 1;
}

.key-hints {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  color: #61768c;
  white-space: nowrap;
}

.key-hints kbd {
  height: 17px;
  min-width: 17px;
  font-size: 8px;
}

.edit-ai {
  top: 56px;
}

.editor-stage {
  position: absolute;
  inset: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 58px 40px 170px;
  pointer-events: none;
}

.crop-shell {
  position: relative;
  flex: none;
  overflow: hidden;
  border-radius: 5px;
  background: #0b1018;
  box-shadow:
    0 42px 120px rgba(0, 0, 0, 0.65),
    0 12px 28px rgba(0, 0, 0, 0.35),
    0 0 0 1px rgba(159, 202, 226, 0.24),
    0 0 0 5px rgba(6, 12, 20, 0.55);
  pointer-events: auto;
  animation: crop-in 0.26s cubic-bezier(0.16, 0.84, 0.24, 1);
}

@keyframes crop-in {
  from {
    opacity: 0;
    transform: scale(0.982);
  }
  to {
    opacity: 1;
    transform: scale(1);
  }
}

.crop-shell::after {
  content: "";
  position: absolute;
  inset: 0;
  border-radius: 4px;
  box-shadow: 0 0 0 1px rgba(255, 255, 255, 0.1) inset;
  pointer-events: none;
  z-index: 4;
}

.base-canvas {
  display: block;
  width: 100%;
  height: 100%;
}

.ann-canvas {
  position: absolute;
  inset: 0;
  display: block;
  width: 100%;
  height: 100%;
  touch-action: none;
  z-index: 3;
}

.cursor-crosshair {
  cursor: crosshair;
}

.cursor-text {
  cursor: text;
}

.cursor-copy {
  cursor: copy;
}

.text-input {
  position: absolute;
  z-index: 8;
  min-width: 130px;
  min-height: 34px;
  max-height: 60%;
  border: 1px dashed;
  border-radius: 6px;
  outline: none;
  resize: none;
  overflow: auto;
  font-family: var(--font-ui);
  font-weight: 600;
  letter-spacing: 0.005em;
  box-shadow: 0 8px 24px rgba(0, 0, 0, 0.4);
  caret-color: currentColor;
  white-space: pre-wrap;
  user-select: text;
  -webkit-user-select: text;
}

.text-input::placeholder {
  color: rgba(255, 255, 255, 0.55);
  font-weight: 400;
}

.working-chip {
  position: fixed;
  top: 62px;
  left: 50%;
  transform: translateX(-50%);
  z-index: 50;
  display: inline-flex;
  align-items: center;
  gap: 8px;
  padding: 7px 12px;
  border: 1px solid rgba(255, 255, 255, 0.1);
  border-radius: 99px;
  background: rgba(9, 16, 26, 0.9);
  color: #a9bed2;
  font-size: 10px;
  backdrop-filter: blur(12px);
  pointer-events: none;
}

.toast {
  position: fixed;
  left: 50%;
  bottom: 88px;
  transform: translateX(-50%);
  z-index: 60;
  display: flex;
  align-items: center;
  gap: 8px;
  max-width: min(620px, calc(100vw - 40px));
  padding: 9px 13px;
  border: 1px solid rgba(255, 255, 255, 0.1);
  border-radius: 9px;
  background: rgba(9, 16, 26, 0.94);
  color: #dbe9f6;
  font-size: 11px;
  line-height: 1.45;
  box-shadow: 0 12px 32px rgba(0, 0, 0, 0.45);
  backdrop-filter: blur(14px);
}

.toast.error {
  border-color: rgba(255, 69, 58, 0.35);
  color: #ffc5c0;
}

.toast.ok {
  border-color: rgba(50, 215, 75, 0.28);
  color: #c8f7d2;
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

.spinner {
  width: 12px;
  height: 12px;
  flex: none;
  border: 1.7px solid rgba(103, 232, 249, 0.2);
  border-top-color: #67e8f9;
  border-radius: 50%;
  animation: spin 0.8s linear infinite;
}

.spinner.large {
  width: 26px;
  height: 26px;
  border-width: 2px;
  margin: 22px auto 6px;
}

@keyframes spin {
  to {
    transform: rotate(360deg);
  }
}

/* State screens */
.state-screen {
  position: absolute;
  inset: 0;
  display: grid;
  place-items: center;
  padding: 30px;
  background:
    radial-gradient(ellipse 55% 45% at 50% 38%, rgba(28, 56, 82, 0.3), transparent 72%),
    #05090f;
}

.state-screen::before {
  content: "";
  position: absolute;
  inset: 0;
  opacity: 0.35;
  background-image: radial-gradient(rgba(129, 168, 199, 0.18) 0.7px, transparent 0.7px);
  background-size: 24px 24px;
  mask-image: radial-gradient(ellipse 60% 60% at 50% 50%, black, transparent 75%);
}

.loading-card,
.result-card {
  position: relative;
  z-index: 2;
  text-align: center;
  animation: card-in 0.28s cubic-bezier(0.16, 0.84, 0.24, 1);
}

@keyframes card-in {
  from {
    opacity: 0;
    transform: translateY(8px);
  }
  to {
    opacity: 1;
    transform: translateY(0);
  }
}

.brand-lockup {
  display: inline-flex;
  align-items: center;
  gap: 10px;
  color: #e9f7ff;
  font-size: 16px;
  font-weight: 700;
  letter-spacing: 0.01em;
}

.brand-name {
  font-size: 19px;
}

.loading-card p {
  margin: 4px 0 0;
  color: #6f879c;
  font-size: 11px;
}

.result-card {
  width: min(460px, calc(100vw - 48px));
  padding: 34px 30px 26px;
  border: 1px solid rgba(151, 198, 227, 0.13);
  border-radius: 18px;
  background:
    linear-gradient(180deg, rgba(20, 33, 48, 0.72), rgba(9, 16, 26, 0.94));
  box-shadow:
    0 34px 90px rgba(0, 0, 0, 0.5),
    0 1px 0 rgba(255, 255, 255, 0.05) inset;
  backdrop-filter: blur(18px);
}

.result-icon {
  width: 54px;
  height: 54px;
  display: grid;
  place-items: center;
  margin: 0 auto 16px;
  border-radius: 50%;
  color: #06141c;
  background: linear-gradient(145deg, #6df2b9, #22c56d);
  box-shadow: 0 8px 24px rgba(34, 197, 109, 0.22);
}

.result-icon.error {
  background: linear-gradient(145deg, #ff938b, #e11d48);
  color: #2c0308;
  box-shadow: 0 8px 24px rgba(225, 29, 72, 0.22);
}

.result-icon.closed {
  background: linear-gradient(145deg, #8fa7be, #52677d);
  color: #071018;
}

.result-card h1 {
  margin: 0;
  color: #eef8ff;
  font-size: 20px;
  font-weight: 700;
  letter-spacing: -0.01em;
}

.result-detail {
  margin: 9px 0 0;
  color: #8da3b8;
  font-size: 11px;
  line-height: 1.6;
  word-break: break-all;
}

.error-text {
  color: #ffb4ae;
  max-height: 200px;
  overflow: auto;
  text-align: left;
  font-family: var(--font-mono);
  font-size: 10px;
  padding: 10px;
  border-radius: 8px;
  background: rgba(255, 69, 58, 0.07);
  border: 1px solid rgba(255, 69, 58, 0.14);
}

.result-actions {
  display: flex;
  justify-content: center;
  gap: 8px;
  margin-top: 22px;
}

.result-btn {
  height: 34px;
  padding: 0 16px;
  border: 1px solid rgba(255, 255, 255, 0.13);
  border-radius: 9px;
  background: rgba(255, 255, 255, 0.055);
  color: #cfdeee;
  font-family: inherit;
  font-size: 11px;
  font-weight: 600;
  cursor: pointer;
  transition: background 0.15s ease;
}

.result-btn:hover {
  background: rgba(255, 255, 255, 0.1);
}

.result-btn.primary {
  border-color: rgba(127, 240, 255, 0.34);
  background: linear-gradient(180deg, #5de0ee, #1cc8e3);
  color: #04141c;
  font-weight: 700;
}

.mock-note {
  display: block;
  margin-top: 16px;
  color: #50677c;
  font-size: 9px;
  letter-spacing: 0.02em;
}

@media (max-width: 760px) {
  .key-hints {
    display: none;
  }

  .editor-stage {
    padding: 54px 20px 170px;
  }
}
</style>
