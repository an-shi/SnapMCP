import type { AppStatus, CaptureEntry } from "./types";

const W = 1600;
const H = 1000;

let cachedScreenshot: string | null = null;

function rounded(
  ctx: CanvasRenderingContext2D,
  x: number,
  y: number,
  w: number,
  h: number,
  r: number,
  fill: string,
  stroke?: string,
) {
  ctx.beginPath();
  ctx.roundRect(x, y, w, h, r);
  ctx.fillStyle = fill;
  ctx.fill();
  if (stroke) {
    ctx.strokeStyle = stroke;
    ctx.lineWidth = 1;
    ctx.stroke();
  }
}

function text(
  ctx: CanvasRenderingContext2D,
  value: string,
  x: number,
  y: number,
  size: number,
  color: string,
  weight = "400",
  align: CanvasTextAlign = "left",
) {
  ctx.fillStyle = color;
  ctx.font = `${weight} ${size}px "Segoe UI","Microsoft YaHei UI","Microsoft YaHei",sans-serif`;
  ctx.textAlign = align;
  ctx.textBaseline = "middle";
  ctx.fillText(value, x, y);
}

function drawSidebar(ctx: CanvasRenderingContext2D) {
  ctx.fillStyle = "#0B1220";
  ctx.fillRect(0, 0, 250, H);
  ctx.fillStyle = "#0F1B2D";
  ctx.fillRect(250, 0, 1, H);
  // Logo
  ctx.fillStyle = "#22D3EE";
  ctx.beginPath();
  ctx.roundRect(28, 26, 34, 34, 9);
  ctx.fill();
  ctx.fillStyle = "#06232C";
  ctx.font = '800 19px "Segoe UI",sans-serif';
  ctx.textAlign = "center";
  ctx.textBaseline = "middle";
  ctx.fillText("O", 45, 44);
  text(ctx, "Orbit Admin", 74, 36, 16, "#F5F9FF", "700");
  text(ctx, "SaaS Console", 74, 54, 11, "#6F87A7", "500");
  // Nav
  const items = [
    ["总览", false],
    ["用户管理", true],
    ["角色权限", false],
    ["组织架构", false],
    ["操作审计", false],
    ["系统设置", false],
  ];
  let y = 120;
  for (const [label, active] of items) {
    if (active) {
      rounded(ctx, 16, y - 17, 218, 36, 8, "rgba(34,211,238,0.12)");
      ctx.fillStyle = "#22D3EE";
      ctx.fillRect(16, y - 17, 3, 36);
    }
    ctx.strokeStyle = active ? "#22D3EE" : "#5B7292";
    ctx.lineWidth = 1.5;
    ctx.strokeRect(32, y - 7, 13, 13);
    text(ctx, String(label), 60, y, 13, active ? "#DDF8FF" : "#91A5BE", active ? "600" : "400");
    y += 46;
  }
  rounded(ctx, 16, 870, 218, 92, 12, "rgba(255,255,255,0.04)", "rgba(255,255,255,0.08)");
  text(ctx, "本月配额", 30, 892, 11, "#7C93B2", "600");
  text(ctx, "68%", 30, 918, 22, "#F5F9FF", "700");
  ctx.fillStyle = "#16263A";
  ctx.beginPath();
  ctx.roundRect(30, 940, 188, 6, 3);
  ctx.fill();
  ctx.fillStyle = "#22D3EE";
  ctx.beginPath();
  ctx.roundRect(30, 940, 128, 6, 3);
  ctx.fill();
}

function drawTopbar(ctx: CanvasRenderingContext2D) {
  ctx.fillStyle = "#FFFFFF";
  ctx.fillRect(251, 0, W - 251, 72);
  ctx.fillStyle = "#EDF1F6";
  ctx.fillRect(251, 71, W - 251, 1);
  text(ctx, "控制台", 292, 36, 12, "#8191A6", "500");
  text(ctx, "/", 348, 36, 12, "#B8C3D2");
  text(ctx, "用户管理", 368, 36, 12, "#27374D", "600");
  rounded(ctx, 1050, 18, 260, 36, 18, "#F3F6FA", "#E6EBF2");
  text(ctx, "搜索用户、邮箱或 ID", 1115, 36, 12, "#93A2B7");
  ctx.strokeStyle = "#8B9AAE";
  ctx.lineWidth = 1.6;
  ctx.beginPath();
  ctx.arc(1075, 36, 5, 0, Math.PI * 2);
  ctx.moveTo(1079, 40);
  ctx.lineTo(1083, 44);
  ctx.stroke();
  ctx.fillStyle = "#EAF2FF";
  ctx.beginPath();
  ctx.arc(1390, 36, 17, 0, Math.PI * 2);
  ctx.fill();
  text(ctx, "林", 1390, 36, 13, "#2563EB", "700", "center");
  text(ctx, "Lina Chen", 1418, 29, 12, "#26364A", "600");
  text(ctx, "管理员", 1418, 44, 10, "#8191A6");
}

function drawContent(ctx: CanvasRenderingContext2D) {
  const left = 292;
  text(ctx, "用户管理", left, 126, 28, "#101B2D", "700");
  text(ctx, "管理团队成员、邀请新成员并控制访问权限。", left, 160, 13, "#7A8AA0");
  rounded(ctx, 1258, 106, 92, 38, 9, "#FFFFFF", "#DCE3EC");
  text(ctx, "导出", 1304, 125, 13, "#42546B", "600", "center");
  rounded(ctx, 1360, 106, 138, 38, 9, "#0A84FF", "#0A84FF");
  text(ctx, "+  新建用户", 1429, 125, 13, "#FFFFFF", "600", "center");

  const cards = [
    ["用户总数", "12,480", "+8.2%", "#EAF3FF", "#0A84FF"],
    ["活跃用户", "9,231", "+4.6%", "#EAFBF1", "#12A150"],
    ["待审核", "48", "-12%", "#FFF7E8", "#D97706"],
    ["本月新增", "312", "+21.4%", "#F3ECFF", "#7C3AED"],
  ];
  cards.forEach((card, i) => {
    const x = left + i * 310;
    rounded(ctx, x, 198, 286, 104, 12, "#FFFFFF", "#E5EAF1");
    text(ctx, card[0] as string, x + 20, 226, 12, "#7A8AA0", "600");
    text(ctx, card[1] as string, x + 20, 258, 26, "#142033", "700");
    rounded(ctx, x + 198, 216, 66, 24, 12, card[3] as string);
    text(ctx, card[2] as string, x + 231, 228, 11, card[4] as string, "600", "center");
    text(ctx, "较上月", x + 20, 283, 10, "#A0AEC0");
  });

  const tableX = left;
  const tableY = 336;
  const tableW = 1208;
  rounded(ctx, tableX, tableY, tableW, 574, 14, "#FFFFFF", "#E5EAF1");
  text(ctx, "全部用户", tableX + 24, tableY + 34, 16, "#152238", "700");
  text(ctx, "共 12,480 位成员", tableX + 110, tableY + 34, 11, "#93A2B7");
  rounded(ctx, tableX + 950, tableY + 18, 100, 32, 8, "#F7F9FC", "#E6EBF2");
  text(ctx, "全部角色", tableX + 1000, tableY + 34, 11, "#53677F", "500", "center");
  rounded(ctx, tableX + 1060, tableY + 18, 124, 32, 8, "#F7F9FC", "#E6EBF2");
  text(ctx, "全部状态", tableX + 1122, tableY + 34, 11, "#53677F", "500", "center");

  ctx.fillStyle = "#F8FAFC";
  ctx.fillRect(tableX + 1, tableY + 68, tableW - 2, 38);
  const cols = [
    ["用户", 24],
    ["邮箱", 300],
    ["角色", 590],
    ["状态", 760],
    ["最近登录", 920],
    ["操作", 1110],
  ];
  cols.forEach(([label, offset]) => {
    text(ctx, label as string, tableX + (offset as number), tableY + 87, 11, "#8A99AD", "600");
  });

  const users: Array<[string, string, string, string, string, string, string]> = [
    ["陈思远", "siyuan.chen@orbit.io", "超级管理员", "正常", "2 分钟前", "#16A34A", "陈"],
    ["王雨桐", "yutong.wang@orbit.io", "运营管理员", "正常", "18 分钟前", "#16A34A", "王"],
    ["Alex Morgan", "alex.m@orbit.io", "数据分析师", "正常", "1 小时前", "#16A34A", "A"],
    ["赵一诺", "yinuo.zhao@orbit.io", "客服专员", "待审核", "昨天 16:42", "#D97706", "赵"],
    ["Maya Patel", "maya.p@orbit.io", "开发者", "已停用", "3 天前", "#94A3B8", "M"],
    ["周子墨", "zimo.zhou@orbit.io", "运营专员", "正常", "刚刚", "#16A34A", "周"],
  ];

  users.forEach((u, i) => {
    const rowY = tableY + 158 + i * 62;
    if (i % 2 === 1) {
      ctx.fillStyle = "#FBFCFE";
      ctx.fillRect(tableX + 1, rowY - 31, tableW - 2, 62);
    }
    ctx.fillStyle = "#EDF1F6";
    ctx.fillRect(tableX + 20, rowY + 31, tableW - 40, 1);
    const avatarColors = ["#DBEAFE", "#FCE7F3", "#EDE9FE", "#FEF3C7", "#E0F2FE", "#DCFCE7"];
    ctx.fillStyle = avatarColors[i % avatarColors.length];
    ctx.beginPath();
    ctx.arc(tableX + 38, rowY, 16, 0, Math.PI * 2);
    ctx.fill();
    text(ctx, u[6], tableX + 38, rowY, 11, "#334155", "700", "center");
    text(ctx, u[0], tableX + 64, rowY - 7, 12, "#172336", "600");
    text(ctx, `ID: USR-${(4821 + i).toString()}`, tableX + 64, rowY + 10, 9, "#9AA8BA");
    text(ctx, u[1], tableX + 300, rowY, 11, "#53677F");
    text(ctx, u[2], tableX + 590, rowY, 11, "#42546B", "500");
    rounded(ctx, tableX + 754, rowY - 11, 66, 22, 11, `${u[5]}18`);
    ctx.fillStyle = u[5];
    ctx.beginPath();
    ctx.arc(tableX + 766, rowY, 2.5, 0, Math.PI * 2);
    ctx.fill();
    text(ctx, u[3], tableX + 775, rowY, 10, u[5], "600");
    text(ctx, u[4], tableX + 920, rowY, 11, "#64748B");
    rounded(ctx, tableX + 1066, rowY - 15, 46, 28, 7, "#F1F5F9");
    text(ctx, "编辑", tableX + 1089, rowY, 10, "#42546B", "500", "center");
    rounded(ctx, tableX + 1122, rowY - 15, 64, 28, 7, "#FFF0EF", "#FFD5D1");
    text(ctx, "删除", tableX + 1154, rowY, 10, "#E11D48", "600", "center");
  });

  // Decorative toast in the lower-left corner of the content area
  rounded(ctx, 300, 890, 330, 66, 10, "#0F1B2D");
  text(ctx, "演示画面", 322, 910, 11, "#7DD3FC", "700");
  text(ctx, "这是 SnapMCP 的浏览器预览背景", 322, 932, 11, "#E2E8F0");
}

export function mockScreenshotDataUrl(): string {
  if (cachedScreenshot) return cachedScreenshot;
  const canvas = document.createElement("canvas");
  canvas.width = W;
  canvas.height = H;
  const ctx = canvas.getContext("2d");
  if (!ctx) return "";
  ctx.fillStyle = "#F4F7FB";
  ctx.fillRect(0, 0, W, H);
  if (typeof ctx.roundRect !== "function") {
    // Very old WebViews: fall back to plain rectangles for the mock only.
    (ctx as unknown as { roundRect: (x: number, y: number, w: number, h: number, r: number) => void }).roundRect =
      function (this: CanvasRenderingContext2D, x: number, y: number, w: number, h: number) {
        this.rect(x, y, w, h);
      };
  }
  drawSidebar(ctx);
  drawTopbar(ctx);
  drawContent(ctx);
  cachedScreenshot = canvas.toDataURL("image/png");
  return cachedScreenshot;
}

const MOCK_SETTINGS = {
  shortcut: "CmdOrCtrl+Shift+X",
  saveDir: "C:\\Users\\Demo\\Pictures\\SnapMCP",
  autoStart: false,
};

export function mockStatus(): AppStatus {
  return {
    version: "0.1.1",
    mcp: {
      running: true,
      port: 37621,
      endpoint: "http://127.0.0.1:37621/mcp",
      tokenRequired: false,
      error: null,
    },
    saveDir: MOCK_SETTINGS.saveDir,
    shortcut: MOCK_SETTINGS.shortcut,
    shortcutLabel: "Ctrl + Shift + X",
    shortcutError: null,
    autoStart: MOCK_SETTINGS.autoStart,
    configDir: "C:\\Users\\Demo\\AppData\\Roaming\\SnapMCP",
    captureCount: 18,
    latestCapture: "C:\\Users\\Demo\\Pictures\\SnapMCP\\SnapMCP_20261118_142233_412.png",
    platform: "windows",
    defaultShortcut: "CmdOrCtrl+Shift+X",
    busy: false,
    appPath: "C:\\Program Files\\SnapMCP\\SnapMCP.exe",
  };
}

export function mockCaptures(): CaptureEntry[] {
  const now = Date.now();
  return [
    {
      name: "SnapMCP_20261118_142233_412.png",
      path: "C:\\Users\\Demo\\Pictures\\SnapMCP\\SnapMCP_20261118_142233_412.png",
      modifiedMs: now - 1000 * 60 * 8,
      size: 482_311,
    },
    {
      name: "SnapMCP_20261118_135902_118.png",
      path: "C:\\Users\\Demo\\Pictures\\SnapMCP\\SnapMCP_20261118_135902_118.png",
      modifiedMs: now - 1000 * 60 * 95,
      size: 720_110,
    },
    {
      name: "SnapMCP_20261117_181044_930.png",
      path: "C:\\Users\\Demo\\Pictures\\SnapMCP\\SnapMCP_20261117_181044_930.png",
      modifiedMs: now - 1000 * 60 * 60 * 18,
      size: 1_248_900,
    },
    {
      name: "SnapMCP_20261117_110233_002.png",
      path: "C:\\Users\\Demo\\Pictures\\SnapMCP\\SnapMCP_20261117_110233_002.png",
      modifiedMs: now - 1000 * 60 * 60 * 29,
      size: 392_881,
    },
  ];
}

export function mockOverlayBootstrap() {
  const mode = new URLSearchParams(window.location.search).get("mode") === "interactive"
    ? ("interactive" as const)
    : ("capture" as const);
  return {
    captureId: "mock-capture",
    imageDataUrl: mockScreenshotDataUrl(),
    width: W,
    height: H,
    scaleFactor: 1,
    display: {
      index: 0,
      name: "Demo Display",
      x: 0,
      y: 0,
      width: W,
      height: H,
      scaleFactor: 1,
      isPrimary: true,
    },
    region: { x: 0, y: 0, width: W, height: H },
    mode,
    hint:
      mode === "interactive"
        ? "AI 请求一张用户列表截图，请在删除按钮上标注后发送"
        : null,
  };
}

export function setMockShortcut(shortcut: string) {
  MOCK_SETTINGS.shortcut = shortcut;
}

export function setMockAutoStart(value: boolean) {
  MOCK_SETTINGS.autoStart = value;
}

export function setMockSaveDir(value: string) {
  MOCK_SETTINGS.saveDir = value;
}
