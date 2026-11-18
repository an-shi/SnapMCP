export const PALETTE = [
  "#FF453A",
  "#FF9F0A",
  "#FFD60A",
  "#32D74B",
  "#0A84FF",
  "#32D2E0",
  "#BF5AF2",
  "#FFFFFF",
];

export const TEXT_BACKGROUND = "#0B1220D9";
export const HIGHLIGHT_ALPHA_HEX = "4D";

export function normalizeHex(input: string): string | null {
  let hex = input.trim().replace(/^#/, "");
  if (hex.length === 3 || hex.length === 4) {
    hex = hex
      .split("")
      .map((c) => c + c)
      .join("");
  }
  if (!/^[0-9a-fA-F]{6}([0-9a-fA-F]{2})?$/.test(hex)) return null;
  return `#${hex.slice(0, 6).toUpperCase()}${hex.length === 8 ? hex.slice(6).toUpperCase() : ""}`;
}

export function withAlpha(input: string, alpha: number): string {
  const normalized = normalizeHex(input);
  if (!normalized) return input;
  const a = Math.round(Math.max(0, Math.min(1, alpha)) * 255)
    .toString(16)
    .padStart(2, "0")
    .toUpperCase();
  return `${normalized.slice(0, 7)}${a}`;
}

export function toCssColor(input: string, fallback = "#FF453A"): string {
  const normalized = normalizeHex(input);
  if (!normalized) return fallback;
  if (normalized.length === 9) return normalized;
  return normalized;
}

export function hexLuminance(input: string): number {
  const normalized = normalizeHex(input);
  if (!normalized) return 0.5;
  const hex = normalized.slice(1, 7);
  const r = parseInt(hex.slice(0, 2), 16) / 255;
  const g = parseInt(hex.slice(2, 4), 16) / 255;
  const b = parseInt(hex.slice(4, 6), 16) / 255;
  return 0.2126 * r + 0.7152 * g + 0.0722 * b;
}

export function readableOn(color: string): string {
  return hexLuminance(color) > 0.62 ? "#08111F" : "#FFFFFF";
}
