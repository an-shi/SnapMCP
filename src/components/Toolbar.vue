<script setup lang="ts">
import { computed } from "vue";
import type { Tool } from "../lib/types";
import { PALETTE, normalizeHex } from "../lib/colors";
import Icon from "./Icon.vue";

const props = defineProps<{
  tool: Tool;
  color: string;
  strokeWidth: number;
  fontSize: number;
  textBackground: boolean;
  canUndo: boolean;
  canRedo: boolean;
  interactive: boolean;
  busy: boolean;
  count: number;
}>();

const emit = defineEmits<{
  (e: "update:tool", value: Tool): void;
  (e: "update:color", value: string): void;
  (e: "update:strokeWidth", value: number): void;
  (e: "update:fontSize", value: number): void;
  (e: "update:textBackground", value: boolean): void;
  (e: "undo"): void;
  (e: "redo"): void;
  (e: "copy"): void;
  (e: "save"): void;
  (e: "saveAs"): void;
  (e: "confirm"): void;
  (e: "cancel"): void;
  (e: "reselect"): void;
}>();

const tools: Array<{ id: Tool; icon: string; label: string; key: string }> = [
  { id: "rect", icon: "rect", label: "矩形框选", key: "R" },
  { id: "ellipse", icon: "ellipse", label: "椭圆框选", key: "E" },
  { id: "arrow", icon: "arrow", label: "箭头", key: "A" },
  { id: "line", icon: "line", label: "直线", key: "L" },
  { id: "pen", icon: "pen", label: "画笔", key: "P" },
  { id: "text", icon: "text", label: "文字", key: "T" },
  { id: "highlight", icon: "highlight", label: "高亮", key: "H" },
  { id: "mosaic", icon: "mosaic", label: "马赛克", key: "M" },
  { id: "step", icon: "step", label: "序号", key: "S" },
];

const widths = [1, 2, 4, 6];
const fontSizes = [16, 22, 30];
const normalizedColor = computed(() => normalizeHex(props.color)?.slice(0, 7) ?? "#FF453A");

function setTool(tool: Tool) {
  emit("update:tool", tool);
}
</script>

<template>
  <div class="toolbar" :class="{ busy }" role="toolbar" aria-label="标注工具">
    <div class="tools-group" aria-label="绘图工具">
      <button
        v-for="item in tools"
        :key="item.id"
        type="button"
        class="tool-btn"
        :class="{ active: tool === item.id }"
        :title="`${item.label}  ${item.key}`"
        :aria-label="item.label"
        :aria-pressed="tool === item.id"
        @click="setTool(item.id)"
      >
        <Icon :name="item.icon" :size="17" />
      </button>
    </div>

    <span class="divider" />

    <div class="color-group" aria-label="颜色">
      <button
        v-for="swatch in PALETTE"
        :key="swatch"
        type="button"
        class="swatch"
        :class="{ active: normalizedColor.toUpperCase() === swatch.toUpperCase() }"
        :style="{ background: swatch }"
        :title="`颜色 ${swatch}`"
        :aria-label="`颜色 ${swatch}`"
        @click="emit('update:color', swatch)"
      />
      <label class="swatch custom" :style="{ background: normalizedColor }" title="自定义颜色">
        <input
          type="color"
          :value="normalizedColor"
          aria-label="自定义颜色"
          @input="emit('update:color', ($event.target as HTMLInputElement).value)"
        />
        <Icon name="plus" :size="10" class="custom-plus" />
      </label>
    </div>

    <span class="divider" />

    <template v-if="tool === 'text'">
      <div class="option-group" aria-label="字号">
        <span class="option-label">字号</span>
        <button
          v-for="size in fontSizes"
          :key="size"
          type="button"
          class="text-size-btn"
          :class="{ active: fontSize === size }"
          :title="`字号 ${size}`"
          @click="emit('update:fontSize', size)"
        >
          {{ size }}
        </button>
      </div>
      <button
        type="button"
        class="bg-toggle"
        :class="{ active: textBackground }"
        title="文字底色"
        @click="emit('update:textBackground', !textBackground)"
      >
        底色
      </button>
    </template>
    <div v-else class="option-group" aria-label="线条粗细">
      <span class="option-label">粗细</span>
      <button
        v-for="width in widths"
        :key="width"
        type="button"
        class="width-btn"
        :class="{ active: strokeWidth === width }"
        :title="`${width} px`"
        :aria-label="`线条粗细 ${width} 像素`"
        @click="emit('update:strokeWidth', width)"
      >
        <span :style="{ width: `${Math.min(18, 4 + width * 2)}px`, height: `${width}px` }" />
      </button>
    </div>

    <span class="divider" />

    <div class="history-group">
      <button
        type="button"
        class="icon-btn"
        :disabled="!canUndo || busy"
        title="撤销  Ctrl+Z"
        aria-label="撤销"
        @click="emit('undo')"
      >
        <Icon name="undo" />
      </button>
      <button
        type="button"
        class="icon-btn"
        :disabled="!canRedo || busy"
        title="重做  Ctrl+Shift+Z"
        aria-label="重做"
        @click="emit('redo')"
      >
        <Icon name="redo" />
      </button>
    </div>

    <span class="count-pill" :title="`${count} 个标注`">{{ count }}</span>

    <span class="divider" />

    <div class="action-group">
      <button
        type="button"
        class="icon-btn"
        title="重新框选"
        aria-label="重新框选"
        :disabled="busy"
        @click="emit('reselect')"
      >
        <Icon name="refresh" />
      </button>

      <template v-if="interactive">
        <button
          type="button"
          class="primary-btn confirm"
          :disabled="busy"
          title="发送给 AI  Ctrl+Enter"
          @click="emit('confirm')"
        >
          <Icon name="check" :size="16" />
          <span>{{ busy ? "处理中…" : "发送给 AI" }}</span>
        </button>
      </template>
      <template v-else>
        <button
          type="button"
          class="icon-btn"
          :disabled="busy"
          title="保存 Ctrl+S"
          aria-label="保存"
          @click="emit('save')"
        >
          <Icon name="save" />
        </button>
        <button
          type="button"
          class="icon-btn"
          :disabled="busy"
          title="另存为 Ctrl+Shift+S"
          aria-label="另存为"
          @click="emit('saveAs')"
        >
          <Icon name="saveAs" />
        </button>
        <button
          type="button"
          class="primary-btn"
          :disabled="busy"
          title="复制到剪贴板 Enter"
          @click="emit('copy')"
        >
          <Icon name="copy" :size="16" />
          <span>{{ busy ? "处理中…" : "复制" }}</span>
        </button>
      </template>

      <button
        type="button"
        class="icon-btn cancel"
        :disabled="busy"
        title="取消 Esc"
        aria-label="取消截图"
        @click="emit('cancel')"
      >
        <Icon name="close" />
      </button>
    </div>
  </div>
</template>

<style scoped>
.toolbar {
  position: fixed;
  left: 0;
  right: 0;
  bottom: 26px;
  z-index: 40;
  display: flex;
  align-items: center;
  gap: 8px;
  width: max-content;
  margin-inline: auto;
  max-width: min(1160px, calc(100vw - 28px));
  padding: 8px 10px;
  flex-wrap: wrap;
  justify-content: center;
  row-gap: 6px;
  overflow: visible;
  background: rgba(10, 16, 26, 0.94);
  border: 1px solid rgba(148, 178, 209, 0.18);
  border-radius: 16px;
  box-shadow:
    0 24px 60px rgba(0, 0, 0, 0.55),
    0 2px 0 rgba(255, 255, 255, 0.04) inset;
  backdrop-filter: blur(18px) saturate(1.25);
  color: #dce8f7;
  font-family: var(--font-ui);
  animation: toolbar-in 0.24s cubic-bezier(0.2, 0.8, 0.2, 1);
}

.toolbar::-webkit-scrollbar {
  display: none;
}

.toolbar.busy {
  opacity: 0.82;
}

@keyframes toolbar-in {
  from {
    opacity: 0;
    transform: translateY(10px) scale(0.98);
  }
  to {
    opacity: 1;
    transform: translateY(0) scale(1);
  }
}

.tools-group,
.color-group,
.history-group,
.option-group,
.action-group {
  display: flex;
  align-items: center;
  gap: 3px;
  flex: none;
}

.color-group {
  gap: 6px;
}

.action-group {
  gap: 4px;
}

.tool-btn,
.icon-btn,
.swatch,
.width-btn,
.text-size-btn,
.bg-toggle {
  -webkit-user-select: none;
  user-select: none;
  font-family: inherit;
  cursor: pointer;
  transition:
    background 0.14s ease,
    border-color 0.14s ease,
    color 0.14s ease,
    transform 0.14s ease;
}

.tool-btn {
  width: 32px;
  height: 32px;
  display: grid;
  place-items: center;
  border: 1px solid transparent;
  border-radius: 8px;
  background: transparent;
  color: #91a5bd;
}

.tool-btn:hover {
  color: #eaf6ff;
  background: rgba(255, 255, 255, 0.065);
}

.tool-btn.active {
  color: #06141d;
  background: linear-gradient(180deg, #63e6f4, #22d3ee);
  border-color: rgba(146, 246, 255, 0.6);
  box-shadow: 0 4px 14px rgba(34, 211, 238, 0.22);
}

.icon-btn {
  width: 30px;
  height: 30px;
  display: grid;
  place-items: center;
  border: 1px solid transparent;
  border-radius: 8px;
  background: transparent;
  color: #9fb1c8;
}

.icon-btn:hover:not(:disabled) {
  color: #ffffff;
  background: rgba(255, 255, 255, 0.07);
}

.icon-btn:disabled {
  opacity: 0.28;
  cursor: default;
}

.icon-btn.cancel:hover:not(:disabled) {
  color: #ffb2ac;
  background: rgba(255, 69, 58, 0.13);
}

.divider {
  width: 1px;
  height: 22px;
  flex: none;
  background: rgba(255, 255, 255, 0.1);
}

.swatch {
  position: relative;
  width: 17px;
  height: 17px;
  flex: none;
  padding: 0;
  border: 2px solid rgba(255, 255, 255, 0.12);
  border-radius: 50%;
  box-shadow: 0 0 0 1px rgba(0, 0, 0, 0.24) inset;
}

.swatch:hover {
  transform: scale(1.14);
}

.swatch.active {
  box-shadow:
    0 0 0 2px #0a101a,
    0 0 0 3.5px rgba(255, 255, 255, 0.75);
}

.swatch.custom {
  display: grid;
  place-items: center;
  overflow: hidden;
  border-color: rgba(255, 255, 255, 0.35);
  background: conic-gradient(#ff453a, #ffd60a, #32d74b, #0a84ff, #bf5af2, #ff453a);
}

.swatch.custom input {
  position: absolute;
  inset: 0;
  opacity: 0;
  cursor: pointer;
}

.custom-plus {
  color: #0c1420;
  filter: drop-shadow(0 0 2px rgba(255, 255, 255, 0.9));
  pointer-events: none;
}

.option-group {
  gap: 2px;
}

.option-label {
  margin: 0 4px 0 2px;
  font-size: 10px;
  font-weight: 600;
  letter-spacing: 0.04em;
  color: #677f9a;
  white-space: nowrap;
}

.width-btn {
  width: 27px;
  height: 27px;
  display: grid;
  place-items: center;
  border: 1px solid transparent;
  border-radius: 7px;
  background: transparent;
}

.width-btn span {
  display: block;
  border-radius: 99px;
  background: #8ea4bd;
}

.width-btn:hover {
  background: rgba(255, 255, 255, 0.06);
}

.width-btn.active {
  background: rgba(34, 211, 238, 0.14);
  border-color: rgba(34, 211, 238, 0.34);
}

.width-btn.active span {
  background: #67e8f9;
}

.text-size-btn {
  min-width: 28px;
  height: 27px;
  padding: 0 6px;
  flex: none;
  border: 1px solid transparent;
  border-radius: 7px;
  background: transparent;
  color: #8fa4bc;
  font-size: 10px;
  font-weight: 600;
  white-space: nowrap;
}

.text-size-btn:hover {
  background: rgba(255, 255, 255, 0.06);
  color: #eaf6ff;
}

.text-size-btn.active {
  color: #67e8f9;
  background: rgba(34, 211, 238, 0.15);
  border-color: rgba(34, 211, 238, 0.32);
}

.bg-toggle {
  height: 27px;
  padding: 0 8px;
  flex: none;
  border: 1px solid rgba(255, 255, 255, 0.09);
  border-radius: 7px;
  background: rgba(255, 255, 255, 0.035);
  color: #8298b3;
  font-size: 10px;
  font-weight: 600;
  white-space: nowrap;
}

.bg-toggle.active {
  color: #dffbff;
  border-color: rgba(34, 211, 238, 0.35);
  background: rgba(34, 211, 238, 0.13);
}

.count-pill {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  min-width: 19px;
  height: 19px;
  padding: 0 5px;
  flex: none;
  border-radius: 99px;
  background: rgba(255, 255, 255, 0.055);
  color: #6d86a1;
  font-size: 10px;
  font-weight: 600;
  font-variant-numeric: tabular-nums;
}

.primary-btn {
  height: 32px;
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 0 13px;
  border: 1px solid rgba(127, 240, 255, 0.38);
  border-radius: 9px;
  background: linear-gradient(180deg, #5de0ee, #1cc8e3);
  color: #04141c;
  font-family: inherit;
  font-size: 12px;
  font-weight: 700;
  flex: none;
  cursor: pointer;
  white-space: nowrap;
  box-shadow:
    0 4px 16px rgba(34, 211, 238, 0.18),
    0 1px 0 rgba(255, 255, 255, 0.5) inset;
  transition:
    filter 0.15s ease,
    transform 0.15s ease;
}

.primary-btn.confirm {
  background: linear-gradient(180deg, #6bf0b4, #22c56d);
  border-color: rgba(146, 255, 199, 0.4);
  box-shadow: 0 4px 16px rgba(34, 197, 109, 0.2);
}

.primary-btn:hover:not(:disabled) {
  filter: brightness(1.06);
  transform: translateY(-1px);
}

.primary-btn:disabled {
  opacity: 0.55;
  cursor: default;
}

@media (max-width: 980px) {
  .toolbar {
    gap: 5px;
  }

  .option-label {
    display: none;
  }

  .color-group {
    gap: 4px;
  }

  .swatch {
    width: 15px;
    height: 15px;
  }
}
</style>
