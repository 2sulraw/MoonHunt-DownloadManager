<!-- Root app shell: wraps everything in naive-ui dark theme provider -->
<template>
  <n-config-provider :theme="currentMode === 'light' ? null : darkTheme">
    <router-view />
  </n-config-provider>
</template>

<script setup lang="ts">
import { darkTheme } from 'naive-ui'
import { useTheme } from './composables/useTheme'
import { useI18n } from './i18n'
import { initGlobalTooltips } from './lib/globalTooltip'

const { currentMode, loadTheme } = useTheme()
const { loadLocale } = useI18n()
// Sync the reactive mode from persisted settings so naive-ui components
// (e.g. the schedule date picker) also follow light/dark mode.
loadTheme()
void loadLocale()
// Default every hover tooltip to the app-wide styled, no-delay preview.
initGlobalTooltips()
</script>

<style>
* {
  margin: 0;
  padding: 0;
  box-sizing: border-box;
  user-select: none;
  -webkit-user-select: none;
}
input, textarea, [contenteditable] {
  user-select: text;
  -webkit-user-select: text;
}
:root {
  --accent: #4fc3f7;
  --accent-rgb: 79, 195, 247;
  --accent-strong: #0078ad;
  --status-active: #4fc3f7;
  --status-complete: #81c784;
  --status-paused: #ffa726;
  --status-error: #ef5350;
  --status-waiting: #64b5f6;
  --status-cancelled: #9e9e9e;
  --toolbar-bg: rgba(0,0,0,0.15);
  --toolbar-border: rgba(255,255,255,0.04);
  --toolbar-active: rgba(255,255,255,0.06);
  --toolbar-active-shadow: 0 1px 4px rgba(0,0,0,0.15);
  --bg-primary: #0a0e1a;
  --bg-surface: #16162a;
  --bg-card: #1e1e32;
  --bg-card-alt: rgba(18, 22, 39, 0.95);
  --text: #ffffff;
  --text-muted: #8c95b4;
  --text-dim: #6b7a9a;
  --border: rgba(255, 255, 255, 0.06);
  --border-strong: rgba(255, 255, 255, 0.08);
  --hover-bg: rgba(255, 255, 255, 0.06);
  --scrollbar-thumb: var(--accent);
  --shadow-sm: 0 2px 8px rgba(0, 0, 0, 0.25);
  --shadow-md: 0 4px 16px rgba(0, 0, 0, 0.35);
  --shadow-lg: 0 8px 32px rgba(0, 0, 0, 0.45);
  --shadow-card: 0 6px 20px rgba(0, 0, 0, 0.3), 0 0 0 1px rgba(255, 255, 255, 0.03);
  --sidebar-width: 72px;
}
.light {
  /* Neutral, paper-white theme (Linear-style): cool gray canvas, pure white
     cards, slate text, crisp hairline borders, soft ambient shadows. */
  --toolbar-bg: rgba(20, 24, 34, 0.04);
  --toolbar-border: rgba(20, 24, 34, 0.06);
  --toolbar-active: #ffffff;
  --toolbar-active-shadow: 0 1px 3px rgba(16, 24, 40, 0.10), 0 0 0 1px rgba(16, 24, 40, 0.05);
  --bg-primary: #f4f5f7;
  --bg-surface: #eef0f3;
  --bg-card: #ffffff;
  --bg-card-alt: rgba(255, 255, 255, 0.97);
  --text: #171923;
  --text-muted: #565d6d;
  --text-dim: #838b9b;
  --status-active: #0078b4;
  --status-complete: #237a2a;
  --status-paused: #a06000;
  --status-error: #d32f2b;
  --status-waiting: #1c72cc;
  --status-cancelled: #6b7280;
  --border: rgba(20, 24, 34, 0.09);
  --border-strong: rgba(20, 24, 34, 0.14);
  --hover-bg: rgba(20, 24, 34, 0.05);
  --scrollbar-thumb: rgba(20, 24, 34, 0.25);
  --shadow-sm: 0 1px 2px rgba(16, 24, 40, 0.05);
  --shadow-md: 0 4px 14px rgba(16, 24, 40, 0.08), 0 1px 3px rgba(16, 24, 40, 0.04);
  --shadow-lg: 0 10px 30px rgba(16, 24, 40, 0.12), 0 4px 10px rgba(16, 24, 40, 0.05);
  --shadow-card: 0 1px 2px rgba(16, 24, 40, 0.04), 0 0 0 1px rgba(20, 24, 34, 0.06);
  --sidebar-width: 72px;
}

/* === Light-mode fix-ups ===
   Several shared components hardcode dark-theme decorations (white-alpha
   fills/hover, dark input fills). On the near-white surfaces they render as
   nothing (or worse, a dark box). These global rules carry `html` so their
   specificity beats the scoped component copies of the same classes. */
html.light .nd-input,
html.light .batch-textarea {
  background: #ffffff;
}
html.light .nd-input:hover,
html.light .batch-textarea:hover {
  border-color: var(--border-strong);
  background: #ffffff;
}
html.light .nd-input:focus,
html.light .batch-textarea:focus {
  background: #ffffff;
}
html.light .nd-icon-btn:hover { background: rgba(20, 24, 34, 0.06); }
html.light .nd-close:hover { background: rgba(211, 47, 47, 0.08); }
html.light .nd-tab:hover:not(.active) { background: rgba(20, 24, 34, 0.05); }
html.light .nd-tab-indicator {
  box-shadow: var(--shadow-sm), inset 0 0 0 1px var(--border);
}
html.light .nd-btn-secondary { background: #ffffff; color: var(--text-muted); }
html.light .nd-btn-secondary:hover:not(.disabled) {
  background: rgba(var(--accent-rgb), 0.06);
}
/* Buttons with hardcoded dark hovers (add-download modal footer). */
html.light .btn-cancel:hover,
html.light .btn-primary:hover { background: var(--hover-bg); }
/* Details-modal proxy inputs (dark charcoal was requested for dark mode). */
html.light .pf-input,
html.light .pf-input:hover,
html.light .pf-input:focus { background: #ffffff; }
html.light .pf-toggle-track { background: var(--hover-bg); }

.linear-dark {
  --bg-primary: #08090a;
  --bg-surface: #0f1011;
  --bg-card: #191a1b;
  --bg-card-alt: rgba(15, 16, 17, 0.95);
  --text: #f7f8f8;
  --text-muted: #d0d6e0;
  --text-dim: #62666d;
  --border: rgba(255, 255, 255, 0.06);
  --border-strong: rgba(255, 255, 255, 0.08);
  --hover-bg: rgba(255, 255, 255, 0.04);
  --scrollbar-thumb: var(--accent);
  --shadow-sm: 0 2px 8px rgba(0, 0, 0, 0.4);
  --shadow-md: 0 4px 16px rgba(0, 0, 0, 0.5);
  --shadow-lg: 0 8px 32px rgba(0, 0, 0, 0.6);
  --shadow-card: 0 4px 20px rgba(0, 0, 0, 0.4), 0 0 0 1px rgba(255, 255, 255, 0.04);
  --sidebar-width: 72px;
}

/* App-wide default tooltip (see lib/globalTooltip.ts). Position is computed by
   JS on hover (flip + clamp to viewport), so the bubble never clips. */
.ttp-note {
  position: fixed;
  left: 0;
  top: 0;
  z-index: 3000;
  width: max-content;
  max-width: 260px;
  padding: 6px 9px;
  border-radius: 8px;
  background: var(--bg-card, #1e1e32);
  border: 1px solid var(--border-strong, rgba(255, 255, 255, 0.12));
  box-shadow: var(--shadow-md, 0 4px 16px rgba(0, 0, 0, 0.35));
  color: var(--accent-strong);
  font-size: 11px;
  font-weight: 400;
  font-family: inherit;
  line-height: 1.45;
  text-align: left;
  text-transform: none;
  letter-spacing: normal;
  white-space: normal;
  /* Long unbreakable strings (e.g. file paths) wrap so the box resizes to fit
     its text instead of bleeding past max-width. */
  overflow-wrap: anywhere;
  word-break: break-word;
  pointer-events: none;
  opacity: 0;
  visibility: hidden;
  transition: opacity 0.08s ease;
}
@font-face {
  font-family: 'Vazirmatn';
  font-style: normal;
  font-display: swap;
  font-weight: 400;
  src: url('./assets/fonts/vazirmatn/Vazirmatn-Regular.woff2') format('woff2');
}
@font-face {
  font-family: 'Vazirmatn';
  font-style: normal;
  font-display: swap;
  font-weight: 500;
  src: url('./assets/fonts/vazirmatn/Vazirmatn-Medium.woff2') format('woff2');
}
@font-face {
  font-family: 'Vazirmatn';
  font-style: normal;
  font-display: swap;
  font-weight: 600;
  src: url('./assets/fonts/vazirmatn/Vazirmatn-SemiBold.woff2') format('woff2');
}
@font-face {
  font-family: 'Vazirmatn';
  font-style: normal;
  font-display: swap;
  font-weight: 700;
  src: url('./assets/fonts/vazirmatn/Vazirmatn-Bold.woff2') format('woff2');
}
@font-face {
  font-family: 'Vazirmatn';
  font-style: normal;
  font-display: swap;
  font-weight: 800;
  src: url('./assets/fonts/vazirmatn/Vazirmatn-ExtraBold.woff2') format('woff2');
}
@font-face {
  font-family: 'Vazirmatn';
  font-style: normal;
  font-display: swap;
  font-weight: 900;
  src: url('./assets/fonts/vazirmatn/Vazirmatn-Black.woff2') format('woff2');
}
body {
  margin: 0;
  background: var(--bg-primary);
  color: var(--text);
  font-family: 'Nunito', -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif;
  overflow: hidden;
}

html[lang='fa'] body,
html[lang='fa'] button,
html[lang='fa'] input,
html[lang='fa'] select,
html[lang='fa'] textarea {
  font-family: 'Vazirmatn', 'Nunito', -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif;
}

/* RTL layout adjustments for fixed sidebars and mirrored chrome. */
html[dir='rtl'] .sidebar {
  border-right: none;
  border-left: 1px solid var(--border);
}
html[dir='rtl'] .settings-sidebar {
  left: auto;
  right: var(--sidebar-width, 70px);
  border-right: none;
  border-left: 1px solid var(--border);
}
html[dir='rtl'] .settings-content {
  margin-left: 0;
  margin-right: 240px;
}
html[dir='rtl'] .custom-dropdown-value,
html[dir='rtl'] .custom-dropdown-item,
html[dir='rtl'] .sidebar-item,
html[dir='rtl'] .menu-label,
html[dir='rtl'] .titlebar-title {
  text-align: right;
}

html[dir='rtl'] .hero-bottom-bar {
  left: 16px;
  right: auto;
}
html[dir='rtl'] .sort-menu {
  left: 0;
  right: auto;
}
html[dir='rtl'] .sort-menu-item,
html[dir='rtl'] .sort-option,
html[dir='rtl'] .context-menu-item,
html[dir='rtl'] .context-menu-label,
html[dir='rtl'] .language-card {
  text-align: right;
}
html[dir='rtl'] .group-header {
  border-left: none;
  border-right: 3px solid var(--group-color, var(--accent));
}
html[dir='rtl'] .group-size,
html[dir='rtl'] .group-limit-badge {
  margin-left: 0;
  margin-right: 6px;
}
html[dir='rtl'] .info-box {
  border-left: none;
  border-right: 3px solid rgba(var(--accent-rgb), 0.3);
  border-radius: 8px 0 0 8px;
}
html[dir='rtl'] .dm-table th,
html[dir='rtl'] .dm-table td,
html[dir='rtl'] .dm-file-table th,
html[dir='rtl'] .dm-file-table td {
  text-align: right;
}
html[dir='rtl'] .input-row.fused input {
  border-radius: 0 8px 8px 0;
  border-right: 1px solid var(--border-strong);
  border-left: none;
}
html[dir='rtl'] .dl-limit-unit {
  border-radius: 8px 0 0 8px;
  border-left: 1px solid var(--border-strong);
  border-right: none;
}
html[dir='rtl'] .nd-input-prefix {
  left: auto;
  right: 12px;
}
html[dir='rtl'] .nd-input-prefixed {
  padding-left: 14px;
  padding-right: 36px;
}
html[dir='rtl'] .nd-input-pr {
  padding-left: 38px;
  padding-right: 14px;
}
html[dir='rtl'] .nd-input-clear {
  left: 8px;
  right: auto;
}
html[dir='rtl'] .nd-input-action {
  left: 6px;
  right: auto;
}
html[dir='rtl'] .nd-path-input {
  padding-left: 36px;
  padding-right: 10px;
}
html[dir='rtl'] .nd-toggle.active .nd-toggle-track,
html[dir='rtl'] .nd-toggle.active .nd-toggle-knob {
  transform: translateX(-16px);
}
html[dir='rtl'] .speed-toggle.on .speed-toggle-knob,
html[dir='rtl'] .toggle.active .toggle-track {
  transform: translateX(-18px);
}
html[dir='rtl'] .mode-toggle.light .mode-toggle-knob {
  transform: translateX(-22px);
}

/* â”€â”€â”€ Global focus reset: ZERO trace on mouse click â”€â”€â”€ */
/* Remove tap highlight on mobile */
* {
  -webkit-tap-highlight-color: transparent;
  -webkit-touch-callout: none;
}

/* Remove focus ring/outline on mouse click, keep for keyboard */
:focus:not(:focus-visible) {
  outline: none !important;
  box-shadow: none !important;
  box-shadow: 0 0 0 0 transparent !important;
  -webkit-focus-ring-color: transparent !important;
}

/* Prevent naive-ui or browser defaults from leaking */
button:focus:not(:focus-visible),
a:focus:not(:focus-visible),
input:focus:not(:focus-visible),
select:focus:not(:focus-visible),
textarea:focus:not(:focus-visible),
[tabindex]:focus:not(:focus-visible),
[contenteditable]:focus:not(:focus-visible) {
  outline: none !important;
  box-shadow: none !important;
  -webkit-focus-ring-color: transparent !important;
}

/* Firefox specific: suppress focus ring on mouse */
:-moz-focusring:not(:focus-visible) {
  outline: none !important;
  box-shadow: none !important;
}

/* Keep Material icon glyphs crisp inside small toolbar buttons. */
.material-icons {
  line-height: 1;
  vertical-align: middle;
  overflow: visible;
}
button .material-icons,
button > .material-icons {
  line-height: 1;
  vertical-align: middle;
  pointer-events: none;
}

::-webkit-scrollbar {
  width: 4px;
  height: 4px;
}
::-webkit-scrollbar-track {
  background: transparent;
}
::-webkit-scrollbar-thumb {
  background: var(--scrollbar-thumb);
  border-radius: 2px;
}

</style>
