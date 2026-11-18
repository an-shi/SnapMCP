export type Point = [number, number];

export interface RectAnnotation {
  type: "rect";
  x: number;
  y: number;
  w: number;
  h: number;
  color: string;
  strokeWidth: number;
  fill?: string;
}

export interface EllipseAnnotation {
  type: "ellipse";
  x: number;
  y: number;
  w: number;
  h: number;
  color: string;
  strokeWidth: number;
  fill?: string;
}

export interface ArrowAnnotation {
  type: "arrow";
  from: Point;
  to: Point;
  color: string;
  strokeWidth: number;
  headSize?: number;
}

export interface LineAnnotation {
  type: "line";
  from: Point;
  to: Point;
  color: string;
  strokeWidth: number;
}

export interface PenAnnotation {
  type: "pen";
  points: Point[];
  color: string;
  strokeWidth: number;
}

export interface TextAnnotation {
  type: "text";
  x: number;
  y: number;
  text: string;
  color: string;
  fontSize: number;
  bgColor: string;
  padding: number;
}

export interface HighlightAnnotation {
  type: "highlight";
  x: number;
  y: number;
  w: number;
  h: number;
  color: string;
}

export interface MosaicAnnotation {
  type: "mosaic";
  x: number;
  y: number;
  w: number;
  h: number;
  blockSize: number;
}

export interface StepAnnotation {
  type: "step";
  x: number;
  y: number;
  index: number;
  color: string;
  size: number;
}

export type Annotation =
  | RectAnnotation
  | EllipseAnnotation
  | ArrowAnnotation
  | LineAnnotation
  | PenAnnotation
  | TextAnnotation
  | HighlightAnnotation
  | MosaicAnnotation
  | StepAnnotation;

export type Tool =
  | "rect"
  | "ellipse"
  | "arrow"
  | "line"
  | "pen"
  | "text"
  | "highlight"
  | "mosaic"
  | "step";

export interface DisplayInfo {
  index: number;
  name: string;
  x: number;
  y: number;
  width: number;
  height: number;
  scaleFactor: number;
  isPrimary: boolean;
}

export interface Region {
  x: number;
  y: number;
  width: number;
  height: number;
}

export interface OverlayBootstrap {
  captureId: string;
  imageDataUrl: string;
  width: number;
  height: number;
  scaleFactor: number;
  display: DisplayInfo;
  region: Region;
  mode: "capture" | "interactive";
  hint: string | null;
}

export interface OverlayFrame extends OverlayBootstrap {
  imageUrl: string;
}

export interface ExportResult {
  ok: boolean;
  action: string;
  saved: boolean;
  copied: boolean;
  canceled: boolean;
  path: string | null;
}

export interface McpStatus {
  running: boolean;
  port: number;
  endpoint: string;
  tokenRequired: boolean;
  error: string | null;
}

export interface AppStatus {
  version: string;
  mcp: McpStatus;
  saveDir: string;
  shortcut: string;
  shortcutLabel: string;
  shortcutError: string | null;
  autoStart: boolean;
  configDir: string;
  captureCount: number;
  latestCapture: string | null;
  platform: string;
  defaultShortcut: string;
  busy: boolean;
  appPath: string;
}

export interface CaptureEntry {
  name: string;
  path: string;
  modifiedMs: number;
  size: number;
}

export interface ToolStyle {
  tool: Tool;
  color: string;
  strokeWidth: number;
  fontSize: number;
  textBackground: boolean;
}

export type ExportAction = "clipboard" | "save" | "saveAs" | "confirm";
