import type {
  Annotation,
  ArrowAnnotation,
  EllipseAnnotation,
  HighlightAnnotation,
  LineAnnotation,
  MosaicAnnotation,
  PenAnnotation,
  RectAnnotation,
  StepAnnotation,
  TextAnnotation,
} from "./types";

export const FONT_STACK =
  '"Segoe UI", "Microsoft YaHei UI", "Microsoft YaHei", "PingFang SC", "Hiragino Sans GB", system-ui, sans-serif';

function roundedRect(
  ctx: CanvasRenderingContext2D,
  x: number,
  y: number,
  w: number,
  h: number,
  radius: number,
) {
  const r = Math.max(0, Math.min(radius, w / 2, h / 2));
  ctx.beginPath();
  ctx.moveTo(x + r, y);
  ctx.lineTo(x + w - r, y);
  ctx.quadraticCurveTo(x + w, y, x + w, y + r);
  ctx.lineTo(x + w, y + h - r);
  ctx.quadraticCurveTo(x + w, y + h, x + w - r, y + h);
  ctx.lineTo(x + r, y + h);
  ctx.quadraticCurveTo(x, y + h, x, y + h - r);
  ctx.lineTo(x, y + r);
  ctx.quadraticCurveTo(x, y, x + r, y);
  ctx.closePath();
}

function drawArrow(ctx: CanvasRenderingContext2D, ann: ArrowAnnotation) {
  const [x1, y1] = ann.from;
  const [x2, y2] = ann.to;
  const dx = x2 - x1;
  const dy = y2 - y1;
  const len = Math.hypot(dx, dy);
  if (len < 1) return;
  const ux = dx / len;
  const uy = dy / len;
  const head = Math.max(
    8,
    Math.min(ann.headSize ?? ann.strokeWidth * 4.6, Math.max(8, len * 0.72)),
  );
  const headWidth = head * 0.52;
  const baseX = x2 - ux * head;
  const baseY = y2 - uy * head;
  ctx.save();
  ctx.strokeStyle = ann.color;
  ctx.fillStyle = ann.color;
  ctx.lineWidth = ann.strokeWidth;
  ctx.lineCap = "round";
  ctx.beginPath();
  ctx.moveTo(x1, y1);
  ctx.lineTo(baseX + ux * 2, baseY + uy * 2);
  ctx.stroke();
  ctx.beginPath();
  ctx.moveTo(x2, y2);
  ctx.lineTo(baseX - uy * headWidth, baseY + ux * headWidth);
  ctx.lineTo(baseX + uy * headWidth, baseY - ux * headWidth);
  ctx.closePath();
  ctx.fill();
  ctx.restore();
}

function drawText(ctx: CanvasRenderingContext2D, ann: TextAnnotation) {
  const text = ann.text.replace(/\s+$/, "");
  if (!text.trim()) return;
  const lines = text.split("\n");
  const lineHeight = ann.fontSize * 1.35;
  ctx.save();
  ctx.font = `600 ${ann.fontSize}px ${FONT_STACK}`;
  const width = Math.max(...lines.map((line) => ctx.measureText(line).width));
  const blockW = width + ann.padding * 2;
  const blockH = lineHeight * lines.length + ann.padding * 2;
  if (ann.bgColor && ann.bgColor !== "none" && ann.bgColor !== "transparent") {
    ctx.fillStyle = ann.bgColor;
    roundedRect(ctx, ann.x, ann.y, blockW, blockH, Math.min(10, ann.padding + 2));
    ctx.fill();
  }
  ctx.fillStyle = ann.color;
  ctx.textBaseline = "top";
  lines.forEach((line, index) => {
    ctx.fillText(
      line,
      ann.x + ann.padding,
      ann.y + ann.padding + index * lineHeight + (lineHeight - ann.fontSize) * 0.32,
    );
  });
  ctx.restore();
}

function drawStep(ctx: CanvasRenderingContext2D, ann: StepAnnotation) {
  const radius = ann.size / 2;
  ctx.save();
  ctx.beginPath();
  ctx.arc(ann.x, ann.y, radius + 1.5, 0, Math.PI * 2);
  ctx.fillStyle = "rgba(0,0,0,0.32)";
  ctx.fill();
  ctx.beginPath();
  ctx.arc(ann.x, ann.y, radius, 0, Math.PI * 2);
  ctx.fillStyle = ann.color;
  ctx.fill();
  ctx.beginPath();
  ctx.arc(ann.x, ann.y, Math.max(0, radius - 2.2), 0, Math.PI * 2);
  ctx.strokeStyle = "rgba(255,255,255,0.92)";
  ctx.lineWidth = Math.max(1.5, ann.size * 0.045);
  ctx.stroke();
  ctx.fillStyle = "#FFFFFF";
  ctx.font = `700 ${Math.round(ann.size * 0.5)}px ${FONT_STACK}`;
  ctx.textAlign = "center";
  ctx.textBaseline = "middle";
  ctx.fillText(String(ann.index), ann.x, ann.y + ann.size * 0.02);
  ctx.restore();
}

function drawMosaic(
  ctx: CanvasRenderingContext2D,
  ann: MosaicAnnotation,
  source?: CanvasImageSource,
) {
  if (!source) return;
  const x = Math.round(Math.min(ann.x, ann.x + ann.w));
  const y = Math.round(Math.min(ann.y, ann.y + ann.h));
  const w = Math.max(1, Math.round(Math.abs(ann.w)));
  const h = Math.max(1, Math.round(Math.abs(ann.h)));
  const block = Math.max(2, Math.round(ann.blockSize));
  const sw = Math.max(1, Math.floor(w / block));
  const sh = Math.max(1, Math.floor(h / block));
  const small = document.createElement("canvas");
  small.width = sw;
  small.height = sh;
  const smallCtx = small.getContext("2d");
  if (!smallCtx) return;
  smallCtx.imageSmoothingEnabled = true;
  try {
    smallCtx.drawImage(source, x, y, w, h, 0, 0, sw, sh);
  } catch {
    return;
  }
  ctx.save();
  ctx.imageSmoothingEnabled = false;
  ctx.drawImage(small, 0, 0, sw, sh, x, y, w, h);
  ctx.restore();
}

export function drawAnnotation(
  ctx: CanvasRenderingContext2D,
  ann: Annotation,
  source?: CanvasImageSource,
) {
  switch (ann.type) {
    case "rect": {
      const r = ann as RectAnnotation;
      const x = Math.min(r.x, r.x + r.w);
      const y = Math.min(r.y, r.y + r.h);
      const w = Math.abs(r.w);
      const h = Math.abs(r.h);
      if (r.fill) {
        ctx.save();
        ctx.fillStyle = r.fill;
        ctx.fillRect(x, y, w, h);
        ctx.restore();
      }
      ctx.save();
      ctx.strokeStyle = r.color;
      ctx.lineWidth = r.strokeWidth;
      ctx.lineJoin = "miter";
      ctx.strokeRect(x, y, w, h);
      ctx.restore();
      break;
    }
    case "ellipse": {
      const e = ann as EllipseAnnotation;
      const rx = Math.abs(e.w) / 2;
      const ry = Math.abs(e.h) / 2;
      const cx = Math.min(e.x, e.x + e.w) + rx;
      const cy = Math.min(e.y, e.y + e.h) + ry;
      ctx.save();
      ctx.beginPath();
      ctx.ellipse(cx, cy, Math.max(1, rx), Math.max(1, ry), 0, 0, Math.PI * 2);
      if (e.fill) {
        ctx.fillStyle = e.fill;
        ctx.fill();
      }
      if (e.strokeWidth > 0) {
        ctx.strokeStyle = e.color;
        ctx.lineWidth = e.strokeWidth;
        ctx.stroke();
      }
      ctx.restore();
      break;
    }
    case "arrow":
      drawArrow(ctx, ann as ArrowAnnotation);
      break;
    case "line": {
      const l = ann as LineAnnotation;
      ctx.save();
      ctx.strokeStyle = l.color;
      ctx.lineWidth = l.strokeWidth;
      ctx.lineCap = "round";
      ctx.beginPath();
      ctx.moveTo(l.from[0], l.from[1]);
      ctx.lineTo(l.to[0], l.to[1]);
      ctx.stroke();
      ctx.restore();
      break;
    }
    case "pen": {
      const p = ann as PenAnnotation;
      if (p.points.length < 2) break;
      ctx.save();
      ctx.strokeStyle = p.color;
      ctx.lineWidth = p.strokeWidth;
      ctx.lineCap = "round";
      ctx.lineJoin = "round";
      ctx.beginPath();
      ctx.moveTo(p.points[0][0], p.points[0][1]);
      for (let i = 1; i < p.points.length; i += 1) {
        ctx.lineTo(p.points[i][0], p.points[i][1]);
      }
      ctx.stroke();
      ctx.restore();
      break;
    }
    case "text":
      drawText(ctx, ann as TextAnnotation);
      break;
    case "highlight": {
      const h = ann as HighlightAnnotation;
      ctx.save();
      ctx.fillStyle = h.color;
      roundedRect(
        ctx,
        Math.min(h.x, h.x + h.w),
        Math.min(h.y, h.y + h.h),
        Math.abs(h.w),
        Math.abs(h.h),
        4,
      );
      ctx.fill();
      ctx.restore();
      break;
    }
    case "mosaic":
      drawMosaic(ctx, ann as MosaicAnnotation, source);
      break;
    case "step":
      drawStep(ctx, ann as StepAnnotation);
      break;
  }
}

export function drawAnnotations(
  ctx: CanvasRenderingContext2D,
  annotations: Annotation[],
  source?: CanvasImageSource,
) {
  for (const ann of annotations) {
    drawAnnotation(ctx, ann, source);
  }
}

export function renderComposite(
  base: HTMLCanvasElement,
  annotations: Annotation[],
): HTMLCanvasElement {
  const out = document.createElement("canvas");
  out.width = base.width;
  out.height = base.height;
  const ctx = out.getContext("2d");
  if (!ctx) return out;
  ctx.drawImage(base, 0, 0);
  ctx.save();
  drawAnnotations(ctx, annotations, base);
  ctx.restore();
  return out;
}

export function draftAnnotation(
  ann: Annotation,
  ctx: CanvasRenderingContext2D,
  source?: CanvasImageSource,
  dpr = 1,
) {
  ctx.save();
  drawAnnotation(ctx, ann, source);
  if (ann.type === "mosaic") {
    ctx.setLineDash([6 * dpr, 4 * dpr]);
    ctx.strokeStyle = "#22D3EE";
    ctx.lineWidth = 1.5 * dpr;
    ctx.strokeRect(ann.x, ann.y, ann.w, ann.h);
  }
  ctx.restore();
}
