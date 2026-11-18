<script setup lang="ts">
withDefaults(defineProps<{ name: string; size?: number }>(), { size: 18 });

const icons: Record<string, string> = {
  camera:
    '<path d="M4 8h3l1.6-2.2h6.8L17 8h3v11H4z"/><circle cx="12" cy="13" r="3.4"/>',
  rect: '<rect x="4" y="5" width="16" height="14" rx="2"/>',
  ellipse: '<ellipse cx="12" cy="12" rx="8.5" ry="6.5"/>',
  arrow: '<path d="M5 19 19 5"/><path d="M11 5h8v8"/>',
  line: '<path d="M5 19 19 5"/><circle cx="5" cy="19" r="1"/><circle cx="19" cy="5" r="1"/>',
  pen: '<path d="M4 20l4.2-1 10.1-10.1a2.12 2.12 0 0 0-3-3L5.2 16z"/><path d="M13.8 7.4l2.8 2.8"/>',
  text: '<path d="M6 5h12M12 5v14M8.5 19h7"/>',
  highlight:
    '<path d="M4 20h16"/><path d="M6.5 16.5l8.8-8.8 3 3-8.8 8.8H6.5z"/><path d="M14 5l2-2 5 5-2 2"/>',
  mosaic:
    '<rect x="4" y="4" width="6" height="6" rx="1"/><rect x="14" y="4" width="6" height="6" rx="1"/><rect x="4" y="14" width="6" height="6" rx="1"/><rect x="14" y="14" width="6" height="6" rx="1"/>',
  step: '<circle cx="12" cy="12" r="8.5"/><path d="M11 9.3l2-1.2V16M9.5 16h5"/>',
  undo: '<path d="M9 7L4 12l5 5"/><path d="M4 12h9.3A5.7 5.7 0 0 1 19 17.7V19"/>',
  redo: '<path d="M15 7l5 5-5 5"/><path d="M20 12h-9.3A5.7 5.7 0 0 0 5 17.7V19"/>',
  copy: '<rect x="8" y="8" width="12" height="12" rx="2"/><path d="M16 8V6a2 2 0 0 0-2-2H6a2 2 0 0 0-2 2v8a2 2 0 0 0 2 2h2"/>',
  save: '<path d="M5 4h11l3 3v13H5z"/><path d="M8 4v6h8V4M8 20v-6h8v6"/>',
  saveAs: '<path d="M5 4h9l3 3v6"/><path d="M5 4v16h8"/><path d="M15 19l5-5M16 14h4v4"/>',
  close: '<path d="M6 6l12 12M18 6L6 18"/>',
  check: '<path d="M4.5 12.5l5 5 10-11"/>',
  settings:
    '<path d="M4 7h9M17 7h3M4 17h3M11 17h9"/><circle cx="15" cy="7" r="2"/><circle cx="9" cy="17" r="2"/>',
  folder: '<path d="M3 7h6l2 2h10v10H3z"/>',
  keyboard:
    '<rect x="3" y="6" width="18" height="12" rx="2"/><path d="M6.5 10h.01M10 10h.01M13.5 10h.01M17 10h.01M6.5 13h.01M10 13h.01M13.5 13h.01M17 13h.01M7 16h10"/>',
  plug: '<path d="M9 3v5M15 3v5"/><path d="M7 8h10v3a5 5 0 0 1-10 0z"/><path d="M12 16v5"/>',
  external: '<path d="M14 4h6v6"/><path d="M20 4l-9 9"/><path d="M18 14v5a1 1 0 0 1-1 1H5a1 1 0 0 1-1-1V7a1 1 0 0 1 1-1h5"/>',
  refresh: '<path d="M20 11a8 8 0 1 0-2.2 6.5"/><path d="M20 4v7h-7"/>',
  shield: '<path d="M12 3l7 3v5c0 4.5-2.9 7.7-7 10-4.1-2.3-7-5.5-7-10V6z"/><path d="M9 12l2 2 4-4"/>',
  bolt: '<path d="M13 2L5 13h6l-1 9 8-11h-6z"/>',
  monitor: '<rect x="3" y="4" width="18" height="12" rx="2"/><path d="M9 20h6M12 16v4"/>',
  image: '<rect x="3" y="4" width="18" height="16" rx="2"/><circle cx="8.5" cy="9" r="1.6"/><path d="M21 15l-5-5L5 20"/>',
  trash: '<path d="M4 7h16M10 11v6M14 11v6"/><path d="M6 7l1 13h10l1-13M9 7V4h6v3"/>',
  chevron: '<path d="M9 5l7 7-7 7"/>',
  plus: '<path d="M12 5v14M5 12h14"/>',
  sparkles:
    '<path d="M12 3l1.7 4.3L18 9l-4.3 1.7L12 15l-1.7-4.3L6 9l4.3-1.7z"/><path d="M19 14l.9 2.1L22 17l-2.1.9L19 20l-.9-2.1L16 17l2.1-.9z"/>',
  search: '<circle cx="11" cy="11" r="6.5"/><path d="M16 16l4 4"/>',
  crosshair: '<circle cx="12" cy="12" r="7"/><path d="M12 2v4M12 18v4M2 12h4M18 12h4"/>',
  mouse: '<rect x="7" y="3" width="10" height="18" rx="5"/><path d="M12 7v4"/>',
  info: '<circle cx="12" cy="12" r="9"/><path d="M12 11v5M12 8h.01"/>',
  clipboard:
    '<rect x="5" y="4" width="14" height="17" rx="2"/><path d="M9 4V3h6v1M9 10h6M9 14h6M9 18h4"/>',
  window: '<rect x="3" y="4" width="18" height="16" rx="2"/><path d="M3 8h18"/>',
  globe: '<circle cx="12" cy="12" r="8.5"/><path d="M3.5 12h17M12 3.5c2.4 2.3 3.6 5.2 3.6 8.5S14.4 18.2 12 20.5C9.6 18.2 8.4 15.3 8.4 12S9.6 5.8 12 3.5z"/>',
  dot: '<circle cx="12" cy="12" r="3" fill="currentColor" stroke="none"/>',
  spinner: '<path d="M12 3a9 9 0 1 0 9 9" opacity=".4"/><path d="M21 12a9 9 0 0 0-9-9"/>',
};

const fallback = icons.info;
</script>

<template>
  <svg
    :width="size"
    :height="size"
    viewBox="0 0 24 24"
    fill="none"
    stroke="currentColor"
    stroke-width="1.7"
    stroke-linecap="round"
    stroke-linejoin="round"
    aria-hidden="true"
    v-html="icons[name] ?? fallback"
  />
</template>
