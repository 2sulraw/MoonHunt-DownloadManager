<template>
  <div
    data-tauri-drag-region
    class="titlebar"
  >
    <span class="titlebar-title" data-tauri-drag-region>{{ title }}</span>

    <!-- Search -->
    <div v-if="searchEnabled" class="titlebar-search-center">
      <div class="search-input-wrap" :class="{ open: showPreview }">
        <i class="material-icons search-icon">search</i>
        <input
          :value="searchQuery"
          @input="onSearchInput(($event.target as HTMLInputElement).value)"
          @focus="searchFocused = true; activeIndex = -1"
          @blur="onSearchBlur"
          @keydown="onSearchKeydown"
          class="search-field"
          :placeholder="effectiveSearchPlaceholder"
          ref="searchInputRef"
          role="combobox"
          :aria-expanded="showPreview"
          aria-autocomplete="list"
        />
        <button v-if="searchQuery" @click="clearSearch" class="search-clear-btn">
          <i class="material-icons">close</i>
        </button>
        <!-- Preview dropdown -->
        <div v-if="showPreview" class="search-preview" role="listbox">
          <template v-if="searchResults.length > 0">
            <button
              v-for="(item, idx) in searchResults"
              :key="item.gid"
              class="search-preview-item"
              :class="{ active: idx === activeIndex }"
              role="option"
              :aria-selected="idx === activeIndex"
              @mousedown.prevent="selectItem(item.gid)"
              @mouseenter="activeIndex = idx"
            >
              <i class="material-icons preview-status-icon" :class="'st-' + item.status">{{ statusIcon(item.status) }}</i>
              <span class="preview-main">
                <span class="preview-name" :title="item.filename" v-html="highlightMatch(item.filename)"></span>
                <span class="preview-sub">
                  <span class="preview-status" :class="'st-' + item.status">{{ item.status }}</span>
                  <span class="preview-progress">{{ item.progress }}%</span>
                </span>
              </span>
            </button>
            <div v-if="searchTotal > searchResults.length" class="search-preview-footer">
              {{ searchTotal - searchResults.length }} {{ t('more — press Enter to filter list') }}
            </div>
          </template>
          <div v-else class="search-preview-empty">
            <i class="material-icons">search_off</i>
            <span>{{ t('No matches for') }} “{{ searchQuery.trim() }}”</span>
          </div>
        </div>
      </div>
    </div>

    <div v-if="showDownloadActions" class="titlebar-actions">
      <button @click="$emit('batch-download')" class="titlebar-action" :title="t('Batch Download')">
        <i class="material-icons" style="font-size: 18px;">playlist_add</i>
      </button>
      <button @click="$emit('new-download')" class="titlebar-action primary" :title="t('New Download')">
        <i class="material-icons" style="font-size: 18px;">add</i>
      </button>
    </div>

    <div class="titlebar-controls">
      <button
        v-if="updateAvailable"
        @click="$emit('update-available-click')"
        class="titlebar-btn update-badge"
        :title="t('Update is Ready')"
        aria-label="Update is Ready"
      >
        <i class="material-icons" style="font-size: 18px;">check_circle</i>
        <span class="update-badge-label">{{ t('Update') }}</span>
        <span class="update-badge-dot"></span>
      </button>
      <button @click="appWindow.minimize()" class="titlebar-btn">
        <i class="material-icons" style="font-size: 18px;">remove</i>
      </button>
      <button @click="appWindow.toggleMaximize()" class="titlebar-btn">
        <i class="material-icons" style="font-size: 18px;">crop_square</i>
      </button>
      <button @click="appWindow.close()" class="titlebar-btn close-btn">
        <i class="material-icons" style="font-size: 18px;">close</i>
      </button>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, onUnmounted, watch, computed } from 'vue'
import { getCurrentWindow } from '@tauri-apps/api/window'
import type { Window } from '@tauri-apps/api/window'
import { useI18n } from '../i18n'

const { t } = useI18n()

export interface SearchPreviewItem {
  gid: string
  filename: string
  url: string
  status: string
  progress: number
}

const props = withDefaults(defineProps<{
  title: string
  searchQuery?: string
  searchEnabled?: boolean
  searchPlaceholder?: string
  showDownloadActions?: boolean
  updateAvailable?: boolean
  searchResults?: SearchPreviewItem[]
  searchTotal?: number
  searchPreviewEnabled?: boolean
}>(), {
  searchQuery: '',
  searchEnabled: false,
  searchPlaceholder: '',
  showDownloadActions: false,
  updateAvailable: false,
  searchResults: () => [],
  searchTotal: 0,
  searchPreviewEnabled: true,
})

const effectiveSearchPlaceholder = computed(() => props.searchPlaceholder || t('Search by name or URL...'))

const emit = defineEmits<{
  'update:searchQuery': [value: string]
  'new-download': []
  'batch-download': []
  'update-available-click': []
  'preview-select': [gid: string]
}>()

const searchInputRef = ref<HTMLInputElement | null>(null)
const searchFocused = ref(false)
const activeIndex = ref(-1)

const showPreview = computed(() =>
  props.searchEnabled && props.searchPreviewEnabled && searchFocused.value && props.searchQuery.trim().length > 0
)

function onSearchInput(value: string) {
  activeIndex.value = -1
  emit('update:searchQuery', value)
}

function clearSearch() {
  activeIndex.value = -1
  emit('update:searchQuery', '')
  searchInputRef.value?.focus()
}

function onSearchBlur() {
  // Delay so mousedown on a preview item fires before we hide.
  setTimeout(() => { searchFocused.value = false }, 120)
}

function selectItem(gid: string) {
  searchFocused.value = false
  activeIndex.value = -1
  emit('preview-select', gid)
}

function onSearchKeydown(e: KeyboardEvent) {
  if (!showPreview.value) return
  if (e.key === 'ArrowDown') {
    e.preventDefault()
    if (!props.searchResults.length) return
    activeIndex.value = (activeIndex.value + 1) % props.searchResults.length
  } else if (e.key === 'ArrowUp') {
    e.preventDefault()
    if (!props.searchResults.length) return
    activeIndex.value = (activeIndex.value - 1 + props.searchResults.length) % props.searchResults.length
  } else if (e.key === 'Enter') {
    if (activeIndex.value >= 0 && props.searchResults[activeIndex.value]) {
      e.preventDefault()
      selectItem(props.searchResults[activeIndex.value].gid)
    } else {
      // Keep filtering the list; just dismiss the dropdown.
      searchFocused.value = false
    }
  }
}

function statusIcon(status: string): string {
  switch (status) {
    case 'active': return 'download'
    case 'waiting': return 'hourglass_top'
    case 'paused': return 'pause'
    case 'complete': return 'done'
    case 'error': return 'error_outline'
    default: return 'inbox'
  }
}

function escapeHtml(s: string): string {
  return s.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;').replace(/"/g, '&quot;')
}

function highlightMatch(name: string): string {
  const q = props.searchQuery.trim()
  if (!q) return escapeHtml(name)
  const idx = name.toLowerCase().indexOf(q.toLowerCase())
  if (idx === -1) return escapeHtml(name)
  return escapeHtml(name.slice(0, idx)) + '<mark>' + escapeHtml(name.slice(idx, idx + q.length)) + '</mark>' + escapeHtml(name.slice(idx + q.length))
}

function handleKeydown(e: KeyboardEvent) {
  if ((e.ctrlKey || e.metaKey) && e.key === 'f') {
    e.preventDefault()
    if (!props.searchEnabled) return
    searchInputRef.value?.focus()
    searchInputRef.value?.select()
  }
  if (e.key === 'Escape' && props.searchQuery) {
    emit('update:searchQuery', '')
  }
}

watch(() => props.searchEnabled, (enabled) => {
  if (!enabled) {
    emit('update:searchQuery', '')
  }
})

onMounted(() => {
  document.addEventListener('keydown', handleKeydown)
})

onUnmounted(() => {
  document.removeEventListener('keydown', handleKeydown)
})

let appWindow: Window | { minimize: () => void; toggleMaximize: () => void; close: () => void } = {
  minimize: () => {},
  toggleMaximize: () => {},
  close: () => {},
}
try {
  appWindow = getCurrentWindow()
} catch {
  // Running in browser (vite dev)
}
</script>

<style scoped>
.titlebar {
  position: relative;
  /* Above page layers (sticky toolbar is z-900) so the search preview
     dropdown — trapped in the translateX(-50%) stacking context below —
     always paints on top. Still below modals/menus (z-1000+). */
  z-index: 950;
  padding: 0 16px;
  height: 50px;
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  background: var(--bg-card);
  border-bottom: 1px solid var(--border);
  flex-shrink: 0;
}

.titlebar-title {
  color: var(--text);
  font-size: 16px;
  font-weight: 500;
  flex-shrink: 0;
}

/* ── Search ── */
.titlebar-search-center {
  position: absolute;
  left: 50%;
  transform: translateX(-50%);
  display: flex;
  align-items: center;
}

.search-input-wrap {
  display: flex;
  align-items: center;
  gap: 6px;
  width: 320px;
  height: 32px;
  background: rgba(255, 255, 255, 0.04);
  border: 1px solid var(--border);
  border-radius: 8px;
  padding: 0 10px;
  transition: border-color 0.2s, box-shadow 0.2s;
}

.search-input-wrap:focus-within {
  border-color: var(--accent-strong);
  background: rgba(var(--accent-rgb), 0.03);
}

.search-icon {
  font-size: 16px;
  color: var(--text-dim);
  flex-shrink: 0;
}

.search-field {
  flex: 1;
  background: none;
  border: none;
  color: var(--text);
  font-size: 13px;
  font-family: inherit;
  outline: none;
  min-width: 0;
}

.search-field::placeholder {
  color: var(--text-dim);
  font-size: 12px;
}

.search-clear-btn {
  background: none;
  border: none;
  color: var(--text-dim);
  cursor: pointer;
  padding: 2px;
  border-radius: 4px;
  display: flex;
  flex-shrink: 0;
  transition: color 0.15s;
}

.search-clear-btn:hover {
  color: var(--text);
}

.search-clear-btn i {
  font-size: 14px;
}

/* ── Search preview dropdown ── */
.search-input-wrap {
  position: relative;
}

.search-input-wrap.open {
  border-bottom-left-radius: 0;
  border-bottom-right-radius: 0;
}

.search-preview {
  position: absolute;
  top: calc(100% + 5px);
  left: -1px;
  right: -1px;
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: 10px;
  box-shadow: 0 12px 32px rgba(0, 0, 0, 0.35);
  overflow: hidden;
  z-index: 1000;
  padding: 4px;
}

.search-preview-item {
  display: flex;
  align-items: center;
  gap: 8px;
  width: 100%;
  padding: 7px 8px;
  background: none;
  border: none;
  border-radius: 7px;
  cursor: pointer;
  text-align: left;
  color: var(--text);
  font-family: inherit;
}

.search-preview-item:hover,
.search-preview-item.active {
  background: var(--hover-bg);
}

.preview-status-icon {
  font-size: 16px;
  flex-shrink: 0;
  color: var(--text-dim);
}

.preview-status-icon.st-active { color: var(--accent-strong); }
.preview-status-icon.st-complete { color: var(--status-complete, #81c784); }
.preview-status-icon.st-error { color: var(--status-error, #e57373); }
.preview-status-icon.st-paused { color: var(--status-paused, #ffb74d); }

.preview-main {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 1px;
}

.preview-name {
  font-size: 12.5px;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.preview-name mark {
  background: rgba(var(--accent-rgb), 0.35);
  color: inherit;
  border-radius: 2px;
  padding: 0 1px;
}

.preview-sub {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 10.5px;
  color: var(--text-dim);
}

.preview-status.st-active { color: var(--accent-strong); }
.preview-status.st-complete { color: var(--status-complete, #81c784); }
.preview-status.st-error { color: var(--status-error, #e57373); }

.search-preview-footer {
  padding: 6px 8px 4px;
  font-size: 11px;
  color: var(--text-dim);
  text-align: center;
}

.search-preview-empty {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 6px;
  padding: 12px 8px;
  font-size: 12px;
  color: var(--text-dim);
}

.search-preview-empty i {
  font-size: 16px;
}

/* ── Actions ── */
.titlebar-actions {
  display: flex;
  align-items: center;
  gap: 4px;
  flex-shrink: 0;
}

.titlebar-action {
  background: none;
  border: none;
  color: var(--text-dim);
  width: 32px;
  height: 32px;
  border-radius: 6px;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: background 0.2s, color 0.2s;
}

.titlebar-action:hover {
  background: var(--hover-bg);
  color: var(--text);
}

.titlebar-action.primary {
  background: rgba(var(--accent-rgb), 0.1);
  color: var(--accent-strong);
}

.titlebar-action.primary:hover {
  background: rgba(var(--accent-rgb), 0.2);
}

/* ── Window Controls ── */
.titlebar-controls {
  display: flex;
  align-items: center;
  gap: 4px;
  flex-shrink: 0;
}

.titlebar-btn {
  background: none;
  border: none;
  color: var(--text-dim);
  width: 36px;
  height: 36px;
  border-radius: 6px;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: background 0.2s, color 0.2s;
}

.titlebar-btn:hover {
  background: var(--hover-bg);
}

.titlebar-btn.close-btn:hover {
  background: var(--hover-bg);
  color: var(--status-error);
}

.update-badge {
  position: relative;
  display: inline-flex;
  align-items: center;
  gap: 4px;
  width: auto;
  padding: 0 10px;
  color: var(--status-complete, #81c784);
  background: rgba(129, 199, 132, 0.10);
  border: 1px solid rgba(129, 199, 132, 0.22);
}

.update-badge-label {
  font-size: 11px;
  font-weight: 600;
}

.update-badge:hover {
  background: rgba(129, 199, 132, 0.16);
  border-color: rgba(129, 199, 132, 0.32);
}

.update-badge-dot {
  position: absolute;
  top: 6px;
  right: 6px;
  width: 6px;
  height: 6px;
  border-radius: 50%;
  background: var(--status-complete, #81c784);
  border: 1.5px solid var(--bg-card);
}
</style>
