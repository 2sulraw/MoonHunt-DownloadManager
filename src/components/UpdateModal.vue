<template>
  <transition name="modal-fade">
    <div
      v-if="visible"
      class="um-overlay"
      @mousedown.self.prevent="overlayPress()"
      @mouseup.self="overlayRelease() && $emit('close')"
      @mouseleave="overlayCancel()"
    >
      <div class="um-modal">
        <button class="um-close" @click="$emit('close')" :aria-label="t('Close')">
          <i class="material-icons">close</i>
        </button>
        <div class="um-hero">
          <div class="um-logo-ring">
            <i class="material-icons">nightlight_round</i>
          </div>
          <h2 class="um-title">{{ t('Update is Ready') }}</h2>
          <p class="um-sub">{{ t('A new version is available') }}</p>
          <span class="um-ver" dir="ltr">{{ update?.version || '…' }}</span>
          <span class="um-ready"><i class="material-icons">check_circle</i> {{ t('Ready') }}</span>
        </div>

        <div class="um-actions">
          <button type="button" class="um-btn um-btn-secondary" @click="$emit('close')">
            {{ t('Later') }}
          </button>
          <button
            type="button"
            class="um-ready-btn"
            @click="downloadAndInstall"
            :disabled="installing"
          >
            <i class="material-icons">{{ installing ? 'hourglass_top' : 'check_circle' }}</i>
            <span>{{ installing ? t('Downloading update…') : t('Install') }}</span>
          </button>
        </div>

        <div v-if="progress && progress.total > 0" class="um-progress">
          <div class="um-progress-track"><div class="um-progress-fill" :style="{ width: progress.percent + '%' }"></div></div>
          <span class="um-progress-text">{{ Math.round(progress.percent) }}%</span>
        </div>

        <p v-if="installError" class="um-error">{{ installError }}</p>

        <div v-if="update?.body" class="um-changelog-section">
          <h4 class="um-changelog-title">{{ t('Changelog') }}</h4>
          <div class="um-changelog-body">{{ update.body }}</div>
        </div>

        <button type="button" class="um-changelog" @click="openChangelog">
          <i class="material-icons">open_in_new</i>
          {{ t('View changelog on GitHub') }}
        </button>
      </div>
    </div>
  </transition>
</template>

<script setup lang="ts">
import { ref, watch } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { useOverlayClose } from '../composables/useOverlayClose'
import { useI18n } from '../i18n'

interface UpdateInfo {
  version?: string
  url?: string
  assetUrl?: string
  body?: string
}

const props = defineProps<{ visible: boolean; update?: UpdateInfo | null }>()
defineEmits<{ close: [] }>()

const { overlayPress, overlayCancel, overlayRelease } = useOverlayClose()
const { t } = useI18n()

const installing = ref(false)
const installError = ref('')
const progress = ref<{ downloaded: number; total: number; percent: number } | null>(null)
let unlistenProgress: UnlistenFn | null = null

watch(() => progress.value, () => {}, { deep: true })

async function downloadAndInstall() {
  if (installing.value) return
  installing.value = true
  try {
    unlistenProgress = await listen('update-download-progress', (e) => {
      progress.value = e.payload as { downloaded: number; total: number; percent: number }
    })
  } catch {
    // progress event optional
  }
  try {
    await invoke('install_update', { assetUrl: props.update?.assetUrl ?? '' })
    progress.value = { downloaded: 0, total: 0, percent: 100 }
    // Fresh download exits the app to replace it; a cached relaunch keeps the
    // app alive so allow another press. Reset after a beat so the button
    // becomes pressable again if the installer was canceled.
    window.setTimeout(() => {
      unlistenProgress?.()
      installing.value = false
    }, 2500)
  } catch (e) {
    installing.value = false
    progress.value = null
    installError.value = typeof e === 'string' ? e : t('Could not install update')
  }
}

function openChangelog() {
  const url = props.update?.url
  if (url) {
    invoke('open_external', { url }).catch(() => {
      window.open(url, '_blank', 'noopener')
    })
  }
}
</script>

<style scoped>
.um-overlay {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.5);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 2000;
  backdrop-filter: blur(4px);
}

.um-modal {
  position: relative;
  width: min(400px, 90vw);
  max-height: 85vh;
  overflow-y: auto;
  border-radius: 16px;
  background: var(--bg-card);
  border: 1px solid var(--border);
  box-shadow: var(--shadow-lg);
  padding: 24px 20px 20px;
}

.um-close {
  position: absolute;
  top: 12px;
  right: 12px;
  width: 28px;
  height: 28px;
  background: var(--hover-bg);
  border: none;
  border-radius: 6px;
  color: var(--text-dim);
  display: flex;
  align-items: center;
  justify-content: center;
  cursor: pointer;
  transition: background 0.15s, color 0.15s;
  z-index: 1;
}

.um-close:hover { color: var(--text); background: var(--hover-bg); }
.um-close .material-icons { font-size: 16px; }

.um-hero {
  display: flex;
  flex-direction: column;
  align-items: center;
  text-align: center;
  padding: 6px 8px 16px;
  gap: 4px;
}

.um-logo-ring {
  width: 52px;
  height: 52px;
  border-radius: 50%;
  background: rgba(var(--accent-rgb), 0.08);
  border: 1.5px solid rgba(var(--accent-rgb), 0.20);
  display: flex;
  align-items: center;
  justify-content: center;
  margin-bottom: 10px;
}

.um-logo-ring .material-icons {
  font-size: 26px;
  color: var(--accent-strong);
}

.um-title { margin: 0; font-size: 19px; font-weight: 700; letter-spacing: -0.01em; }

.um-sub { color: var(--text-muted); font-size: 13px; line-height: 1.5; margin: 4px 0 0; }

.um-ver {
  font-size: 11px;
  color: var(--accent-strong);
  background: rgba(var(--accent-rgb), 0.06);
  padding: 2px 9px;
  border-radius: 999px;
  border: 1px solid rgba(var(--accent-rgb), 0.10);
  margin-top: 6px;
}

.um-ready {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  margin-top: 8px;
  padding: 3px 10px;
  border-radius: 999px;
  font-size: 12px;
  font-weight: 600;
  color: var(--status-complete, #81c784);
  background: rgba(129, 199, 132, 0.08);
  border: 1px solid rgba(129, 199, 132, 0.18);
}

.um-ready .material-icons { font-size: 14px; }

.um-actions {
  display: flex;
  gap: 10px;
  margin-top: 6px;
}

.um-btn {
  flex: 1;
  height: 38px;
  border-radius: 10px;
  border: 1px solid var(--border);
  background: var(--hover-bg);
  color: var(--text);
  font: inherit;
  font-size: 13px;
  font-weight: 600;
  cursor: pointer;
  transition: background 0.15s, border-color 0.15s, opacity 0.15s;
}

.um-btn:hover:not([disabled]) { border-color: rgba(var(--accent-rgb), 0.30); }

.um-btn[disabled] { opacity: 0.6; cursor: default; }

.um-ready-btn {
  flex: 1;
  height: 38px;
  border-radius: 10px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 6px;
  font-size: 13px;
  font-weight: 600;
  cursor: pointer;
  color: var(--status-complete, #81c784);
  background: rgba(129, 199, 132, 0.08);
  border: 1px solid rgba(129, 199, 132, 0.18);
  transition: background 0.15s, border-color 0.15s, opacity 0.15s;
}

.um-ready-btn:hover:not([disabled]) {
  background: rgba(129, 199, 132, 0.14);
  border-color: rgba(129, 199, 132, 0.3);
}

.um-ready-btn .material-icons { font-size: 16px; }
.um-ready-btn[disabled] { opacity: 0.6; cursor: default; }

.um-progress {
  margin-top: 14px;
  display: flex;
  align-items: center;
  gap: 10px;
}

.um-progress-track {
  flex: 1;
  height: 6px;
  border-radius: 999px;
  background: var(--hover-bg);
  overflow: hidden;
}

.um-progress-fill {
  height: 100%;
  border-radius: 999px;
  background: var(--accent-strong);
  transition: width 0.2s;
}

.um-progress-text { color: var(--text-dim); font-size: 12px; min-width: 34px; text-align: right; }

.um-error {
  margin: 12px 0 0;
  padding: 8px 10px;
  border-radius: 8px;
  background: rgba(244, 67, 54, 0.08);
  border: 1px solid rgba(244, 67, 54, 0.18);
  color: var(--status-error);
  font-size: 12px;
  line-height: 1.45;
}

.um-changelog-section { margin-top: 12px; width: 100%; }
.um-changelog-title { font-size: 12px; font-weight: 600; color: var(--text); margin: 0 0 6px; }
.um-changelog-body { font-size: 12px; color: var(--text-dim); white-space: pre-wrap; max-height: 160px; overflow-y: auto; padding: 10px; background: var(--hover-bg); border-radius: 8px; line-height: 1.5; }

.um-changelog {
  margin-top: 16px;
  width: 100%;
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 6px;
  background: none;
  border: none;
  color: var(--text-dim);
  font: inherit;
  font-size: 12px;
  cursor: pointer;
  padding: 8px;
  border-radius: 8px;
  transition: color 0.15s, background 0.15s;
}

.um-changelog:hover { color: var(--accent-strong); background: rgba(var(--accent-rgb), 0.04); }
.um-changelog .material-icons { font-size: 15px; }
</style>