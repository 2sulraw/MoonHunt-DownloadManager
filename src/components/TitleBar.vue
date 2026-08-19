<template>
  <div
    data-tauri-drag-region
    class="titlebar"
  >
    <span class="titlebar-title" data-tauri-drag-region>{{ title }}</span>

    <!-- Search -->
    <div v-if="searchEnabled" class="titlebar-search-center">
      <div class="search-input-wrap">
        <i class="material-icons search-icon">search</i>
        <input
          :value="searchQuery"
          @input="$emit('update:searchQuery', ($event.target as HTMLInputElement).value)"
          class="search-field"
          :placeholder="searchPlaceholder"
          ref="searchInputRef"
        />
        <button v-if="searchQuery" @click="$emit('update:searchQuery', '')" class="search-clear-btn">
          <i class="material-icons">close</i>
        </button>
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

const props = withDefaults(defineProps<{
  title: string
  searchQuery?: string
  searchEnabled?: boolean
  searchPlaceholder?: string
  showDownloadActions?: boolean
  updateAvailable?: boolean
}>(), {
  searchQuery: '',
  searchEnabled: false,
  searchPlaceholder: '',
  showDownloadActions: false,
  updateAvailable: false,
})

const effectiveSearchPlaceholder = computed(() => props.searchPlaceholder || t('Search by name or URL...'))

const emit = defineEmits<{
  'update:searchQuery': [value: string]
  'new-download': []
  'batch-download': []
  'update-available-click': []
}>()

const searchInputRef = ref<HTMLInputElement | null>(null)

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
