<template>
  <transition name="modal-fade">
    <div v-if="isOpen" class="nd-overlay" @mousedown.self.prevent="overlayPress()" @mouseup.self="overlayRelease() && close()" @mouseleave="overlayCancel()">
      <transition name="nd-slide">
        <div class="nd-modal" @click.stop>
          <div class="nd-glow"></div>

          <header class="nd-header">
            <div class="nd-header-left">
              <div class="nd-avatar">
                <i class="material-icons">cloud_download</i>
              </div>
              <div class="nd-titles">
                <h1 class="nd-title">{{ t('New Download') }}</h1>
                <span class="nd-subtitle">{{ t('Paste a link and start downloading') }}</span>
              </div>
            </div>
            <div class="nd-header-actions">
              <ProxyUrlCheckBadge :url="urlValue" :headers="activeHeaders" />
              <button class="nd-icon-btn nd-close" @click="close" :aria-label="t('Close')" :title="t('Close (Esc)')">
                <i class="material-icons">close</i>
              </button>
            </div>
          </header>

          <div class="nd-divider"></div>

          <div class="nd-body">
            <div class="nd-section">
              <div class="nd-section-header">
                <h3 class="nd-section-title">{{ t('Source') }}</h3>
                <span v-if="detectedHost" class="nd-source-badge">{{ detectedHost }}</span>
              </div>
              <div class="nd-input-wrap">
                <input v-model="urlValue" type="url" class="nd-input" :placeholder="t('URL placeholder')" @keyup.enter="submit" autofocus />
                <button v-if="urlValue" @click="urlValue = ''" class="nd-icon-btn nd-input-clear" :aria-label="t('Clear URL')">
                  <i class="material-icons">close</i>
                </button>
              </div>
            </div>

            <Transition name="nd-slide">
              <div v-if="duplicateDownload" class="nd-duplicate-warning">
                <div class="nd-duplicate-icon">
                  <i class="material-icons">warning</i>
                </div>
                <div class="nd-duplicate-info">
                  <span class="nd-duplicate-title">{{ t('Duplicate URL detected') }}</span>
                  <button
                    class="nd-dupe-toggle"
                    role="switch"
                    :aria-checked="allowDuplicate"
                    :aria-label="t('Download as duplicate')"
                    @click="allowDuplicate = !allowDuplicate"
                  >
                    <span class="nd-dupe-track" :class="{ on: allowDuplicate }">
                      <span class="nd-dupe-thumb"></span>
                    </span>
                  </button>
                </div>
              </div>
            </Transition>

            <div class="nd-section">
              <div class="nd-section-header">
                <h3 class="nd-section-title">{{ t('Save As') }}</h3>
              </div>
              <div class="nd-filename-row">
                <div class="nd-input-wrap nd-filename-wrap">
                  <input v-model="filename" type="text" class="nd-input nd-filename-input" :placeholder="t('Auto-detect from URL')" @keyup.enter="submit" />
                  <button v-if="filename" @click="filename = ''" class="nd-icon-btn nd-input-clear" :aria-label="t('Clear filename')">
                    <i class="material-icons">close</i>
                  </button>
                </div>
              </div>
            </div>

            <div class="nd-section">
              <div class="nd-section-header">
                <h3 class="nd-section-title">{{ t('Save Location') }}</h3>
              </div>
              <div class="nd-input-wrap">
                <input v-model="savePath" type="text" class="nd-input nd-path-input" :placeholder="t('Save path placeholder')" @keyup.enter="submit" />
                <button @click="browseFolder" class="nd-input-action" :aria-label="t('Browse folders')" :title="t('Browse folder')">
                  <i class="material-icons">folder_open</i>
                </button>
              </div>
            </div>

            <div class="nd-section">
              <div class="nd-section-header">
                <h3 class="nd-section-title">{{ t('Connections') }}</h3>
              </div>
              <ConnectionSelector v-model="connections" @enter="submit" />
            </div>

            <div class="nd-section">
              <CategoryPicker
                :enabled="categoriesEnabled"
                :mode="categoryMode"
                :selected="selectedCategory"
                :filename="filename || detectedFilename || urlValue"
                @update:enabled="categoriesEnabled = $event; categoryError = ''"
                @update:mode="categoryMode = $event; categoryError = ''"
                @update:selected="selectedCategory = $event; categoryError = ''"
              />
              <div v-if="categoryError" class="nd-category-error">
                <i class="material-icons">info</i>
                <span>{{ categoryError }}</span>
              </div>
            </div>

            <div class="nd-section">
              <SchedulePicker
                v-model:enabled="scheduleEnabled"
                v-model:schedule="pendingSchedule"
              />
            </div>

            <div class="nd-section">
              <ProxyField v-model="downloadProxy" />
            </div>
          </div>

          <footer class="nd-footer">
            <button @click="close" class="nd-btn nd-btn-secondary">
              <i class="material-icons">close</i> {{ t('Cancel') }}
            </button>
            <button @click="submit" :disabled="isAccepting || !urlValue.trim()" class="nd-btn nd-btn-primary" :class="{ disabled: isAccepting || !urlValue.trim() }">
              <i v-if="isAccepting" class="material-icons nd-spinner">hourglass_top</i>
              <i v-else class="material-icons">cloud_download</i>
              <span>{{ isAccepting ? t('Starting...') : t('Start Download') }}</span>
            </button>
          </footer>
        </div>
      </transition>
    </div>
  </transition>
</template>

<script setup lang="ts">
import { ref, computed, watch, onMounted, onBeforeUnmount } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { open } from '@tauri-apps/plugin-dialog'
import ConnectionSelector from './ConnectionSelector.vue'
import CategoryPicker from './CategoryPicker.vue'
import SchedulePicker from './SchedulePicker.vue'
import ProxyUrlCheckBadge from './ProxyUrlCheckBadge.vue'
import ProxyField from './ProxyField.vue'
import { useOverlayClose } from '../composables/useOverlayClose'
import { loadDownloadCategories } from '../composables/useDownloadCategories'
import { resolveCategoryDir, type DownloadCategoryMode } from '../utils/downloadCategories'
import { emptyProxyForm, serializeProxy, type ProxyFormValue } from '../utils/proxy'
import { useI18n } from '../i18n'
import type { ScheduleResult } from './ScheduleModal.vue'
import type { Download } from '../types/download'

const props = defineProps<{
  downloads?: Download[]
}>()

const { overlayPress, overlayCancel, overlayRelease } = useOverlayClose()
const { t } = useI18n()

const isOpen = ref(false)
const urlValue = ref('')
const filename = ref('')
const savePath = ref('')
const connections = ref(16)
const defaultDir = ref('')
const categoriesEnabled = ref(false)
const categoryMode = ref<DownloadCategoryMode>('formatted')
const selectedCategory = ref('')
const categoryError = ref('')
const isAccepting = ref(false)
const allowDuplicate = ref(false)
const scheduleEnabled = ref(false)
const pendingSchedule = ref<ScheduleResult | null>(null)
const activeHeaders = ref<Record<string, string> | null>(null)
const downloadProxy = ref<ProxyFormValue>(emptyProxyForm())

const detectedFilename = computed(() => {
  const u = urlValue.value.trim()
  if (!u) return ''
  try {
    const parsed = new URL(u)
    const name = decodeURIComponent(parsed.pathname.split('/').pop() || '')
    return name && name.includes('.') && !name.endsWith('/') ? name : ''
  } catch {
    return ''
  }
})

let lastAutoFilledFilename = ''
watch(urlValue, () => {
  if (!urlValue.value.trim()) {
    filename.value = ''
    lastAutoFilledFilename = ''
    return
  }
  const detected = detectedFilename.value
  if (!detected) return
  if (!filename.value || filename.value === lastAutoFilledFilename) {
    filename.value = detected
    lastAutoFilledFilename = detected
  }
})

const detectedHost = computed(() => {
  if (!urlValue.value.trim()) return ''
  try {
    return new URL(urlValue.value).hostname.replace('www.', '')
  } catch {
    return ''
  }
})

const duplicateDownload = computed(() => {
  if (!urlValue.value.trim()) return null
  const normalizedUrl = urlValue.value.trim().toLowerCase()
  return (props.downloads || []).find((d) => d.url && d.url.toLowerCase() === normalizedUrl) || null
})

interface DownloadDialogResult {
  accepted: boolean
  filename: string
  filePath: string
  connections: number
  schedule?: ScheduleResult | null
  proxy?: string | null
}

let resolveDownload: ((result: DownloadDialogResult) => void) | null = null

async function loadDefaultDir() {
  if (defaultDir.value) return defaultDir.value
  try {
    defaultDir.value = await invoke<string>('get_default_download_dir')
  } catch {
    defaultDir.value = ''
  }
  return defaultDir.value
}

async function browseFolder() {
  try {
    const selected = await open({ directory: true, multiple: false, defaultPath: savePath.value || 'C:\\Downloads', title: t('Select Download Folder') })
    if (selected) savePath.value = selected as string
  } catch (e) {
    console.error('Failed to select folder:', e)
  }
}

async function openDialog(url: string, suggestedFilename: string, suggestedPath = '', headers?: Record<string, string>): Promise<DownloadDialogResult> {
  urlValue.value = url
  filename.value = suggestedFilename
  savePath.value = suggestedPath
  activeHeaders.value = headers || null
  connections.value = 16
  categoriesEnabled.value = false
  categoryMode.value = 'formatted'
  selectedCategory.value = ''
  categoryError.value = ''
  scheduleEnabled.value = false
  pendingSchedule.value = null
  downloadProxy.value = emptyProxyForm()
  isAccepting.value = false
  allowDuplicate.value = false
  isOpen.value = true

  void loadDownloadCategories()
  void loadDefaultDir()

  const promise = new Promise<DownloadDialogResult>((resolve) => {
    resolveDownload = resolve
  })
  return promise
}

/** Append a `(1)`-style counter to a filename so a duplicate download keeps a distinct target. */
function withDuplicateSuffix(name: string, count = 1): string {
  const dot = name.lastIndexOf('.')
  if (dot <= 0) return `${name} (${count})`
  return `${name.slice(0, dot)} (${count})${name.slice(dot)}`
}

async function submit() {
  if (!urlValue.value.trim() || isAccepting.value) return
  if (categoriesEnabled.value && categoryMode.value === 'manual' && !selectedCategory.value.trim()) {
    categoryError.value = t('Choose a manual category or turn Categories off.')
    return
  }
  isAccepting.value = true
  try {
    const baseDir = savePath.value.trim() || (await loadDefaultDir())
    if (categoriesEnabled.value && !baseDir) {
      categoryError.value = t('Could not resolve the download folder.')
      return
    }
    let finalName = filename.value || detectedFilename.value || urlValue.value
    if (duplicateDownload.value && allowDuplicate.value) {
      finalName = withDuplicateSuffix(finalName, 1)
    }
    const categoryDir = resolveCategoryDir({
      enabled: categoriesEnabled.value,
      mode: categoryMode.value,
      selected: selectedCategory.value,
      filename: finalName,
      baseDir,
    })
    resolveDownload?.({
      accepted: true,
      filename: finalName,
      filePath: categoryDir || savePath.value,
      connections: Math.max(1, Number(connections.value) || 1),
      schedule: scheduleEnabled.value ? pendingSchedule.value : null,
      proxy: serializeProxy(downloadProxy.value),
    })
    isOpen.value = false
  } finally {
    isAccepting.value = false
  }
}

function close() {
  if (!resolveDownload) return
  resolveDownload({
    accepted: false,
    filename: filename.value,
    filePath: savePath.value,
    connections: 1,
  })
  isOpen.value = false
  activeHeaders.value = null
}

function handleKeydown(e: KeyboardEvent) {
  if (e.key === 'Escape' && isOpen.value) close()
}

onMounted(() => {
  document.addEventListener('keydown', handleKeydown)
})

onBeforeUnmount(() => {
  document.removeEventListener('keydown', handleKeydown)
})

defineExpose({ openDialog, isDialogOpen: () => isOpen.value })
</script>

<style scoped>
.nd-overlay {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.55);
  backdrop-filter: blur(4px);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 9999;
  padding: 24px;
}
.nd-slide-enter-active { transition: opacity 0.15s ease; }
.nd-slide-leave-active { transition: opacity 0.1s ease; }
.nd-slide-enter-from, .nd-slide-leave-to { opacity: 0; }

.nd-glow {
  position: absolute; top: -60px; left: 50%; transform: translateX(-50%);
  width: 220px; height: 130px;
  background: radial-gradient(ellipse, rgba(var(--accent-rgb), 0.18), transparent 70%);
  pointer-events: none; z-index: 0; filter: blur(20px);
}

.nd-modal {
  position: relative;
  background: var(--bg-card);
  border: 1px solid var(--border-strong);
  border-radius: 14px;
  box-shadow: var(--shadow-lg);
  max-width: 480px;
  width: 100%;
  max-height: 85vh;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

.nd-header {
  position: relative;
  z-index: 1;
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 14px 16px 10px;
  border-bottom: 1px solid var(--border);
  gap: 12px;
}
.nd-header-left { display: flex; align-items: center; gap: 10px; min-width: 0; flex: 1; }
.nd-avatar {
  width: 36px; height: 36px; border-radius: 10px;
  background: linear-gradient(135deg, rgba(var(--accent-rgb), 0.12), rgba(var(--accent-rgb), 0.04));
  display: flex; align-items: center; justify-content: center;
  flex-shrink: 0; border: 1px solid rgba(var(--accent-rgb), 0.12);
}
.nd-avatar i { font-size: 18px; color: var(--accent-strong); }
.nd-titles { min-width: 0; }
.nd-title { margin: 0; font-size: 14px; font-weight: 700; color: var(--text); letter-spacing: -0.01em; line-height: 1.2; }
.nd-subtitle { font-size: 11px; color: var(--text-muted); font-weight: 400; margin-top: 2px; display: block; }
.nd-header-actions { display: flex; align-items: center; gap: 6px; flex-shrink: 0; }
.nd-icon-btn {
  position: relative; z-index: 1;
  background: none; border: none; color: var(--text-dim);
  width: 28px; height: 28px; border-radius: 7px;
  cursor: pointer; display: flex; align-items: center; justify-content: center;
  transition: all 0.12s ease; flex-shrink: 0; padding: 0;
}
.nd-icon-btn i { font-size: 16px; }
.nd-icon-btn:hover { background: rgba(255,255,255,0.06); color: var(--text); }
.nd-close:hover { background: rgba(239,83,80,0.1); color: var(--status-error); }

.nd-proxy-badge {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  padding: 4px 8px;
  border-radius: 100px;
  font-size: 10px;
  font-weight: 700;
  letter-spacing: 0.03em;
  color: var(--accent-strong);
  background: rgba(var(--accent-rgb), 0.07);
  border: 1px solid rgba(var(--accent-rgb), 0.16);
  white-space: nowrap;
}
.nd-proxy-badge.direct {
  color: var(--text-dim);
  background: var(--hover-bg);
  border-color: var(--border);
}
.nd-proxy-dot {
  width: 5px; height: 5px; border-radius: 50%;
  background: var(--accent);
  box-shadow: 0 0 4px rgba(var(--accent-rgb), 0.4);
}
.nd-proxy-badge.direct .nd-proxy-dot {
  background: var(--text-dim);
  box-shadow: none;
}

.nd-divider {
  height: 1px;
  background: linear-gradient(90deg, transparent, var(--border-strong), transparent);
  margin: 0 16px;
}

.nd-body {
  position: relative; z-index: 1;
  flex: 1; overflow-y: auto;
  padding: 12px 16px 8px;
  display: flex; flex-direction: column; gap: 12px;
}

.nd-section { display: flex; flex-direction: column; gap: 6px; }
.nd-section-header { display: flex; align-items: center; justify-content: space-between; gap: 8px; }
.nd-section-title {
  font-size: 10px; font-weight: 700; text-transform: uppercase;
  letter-spacing: 0.05em; color: var(--text-dim); margin: 0; opacity: 0.7;
}
.nd-source-badge {
  font-size: 9px; font-weight: 600; color: var(--accent-strong);
  background: rgba(var(--accent-rgb), 0.08); padding: 2px 6px;
  border-radius: 100px; border: 1px solid rgba(var(--accent-rgb), 0.12);
  white-space: nowrap; flex-shrink: 0;
}

.nd-input-wrap { position: relative; display: flex; align-items: center; }
.nd-input {
  width: 100%; height: 36px;
  background: rgba(255, 255, 255, 0.025);
  border: 1px solid var(--border); color: var(--text);
  padding: 0 10px; border-radius: 8px; font-size: 12px; font-family: inherit;
  outline: none; transition: all 0.15s ease; box-sizing: border-box;
}
.nd-input:hover { border-color: rgba(255,255,255,0.1); background: rgba(255,255,255,0.035); }
.nd-input:focus { border-color: var(--accent-strong); background: rgba(var(--accent-rgb), 0.03); }
.nd-input::placeholder { color: var(--text-dim); opacity: 0.55; }
.nd-input-wrap:hover .nd-input-clear,
.nd-input:focus ~ .nd-input-clear { opacity: 1; pointer-events: auto; }
.nd-input-clear {
  position: absolute; right: 8px; width: 24px; height: 24px;
  border-radius: 7px; background: transparent; border: none;
  color: var(--text-muted); cursor: pointer;
  display: flex; align-items: center; justify-content: center;
  opacity: 0; pointer-events: none; transition: all 0.12s ease; padding: 0;
}
.nd-input-clear i { font-size: 13px; }
.nd-input-clear:hover { background: rgba(var(--accent-rgb), 0.15); color: var(--text); }
.nd-input-action {
  position: absolute;
  right: 10px;
  width: 26px;
  height: 26px;
  border-radius: 7px;
  background: transparent;
  border: none;
  color: var(--text-muted);
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: all 0.15s ease;
}
.nd-input-action i { font-size: 18px; }
.nd-input-action:hover { background: rgba(var(--accent-rgb), 0.1); color: var(--accent-strong); }

.nd-filename-row { position: relative; }
.nd-filename-wrap { width: 100%; }
.nd-filename-input { padding-right: 34px; }

.nd-detected-row { margin-top: -8px; }
.nd-detected-chip {
  display: inline-flex; align-items: center; gap: 5px;
  font-size: 12px; color: var(--text-dim);
  background: var(--bg-surface); border: 1px solid var(--border);
  border-radius: 8px; padding: 5px 10px; cursor: pointer;
  transition: all 0.15s ease;
}
.nd-detected-chip:hover {
  border-color: rgba(var(--accent-rgb), 0.3);
  color: var(--text-muted);
  background: rgba(var(--accent-rgb), 0.06);
}
.nd-detected-chip .material-icons { font-size: 14px; color: var(--accent-strong); }
.nd-detected-hint {
  font-size: 10px; color: var(--accent-strong);
  font-weight: 600; text-transform: uppercase; letter-spacing: 0.04em;
}

.nd-input-row { display: flex; gap: 6px; align-items: center; }
.nd-input-row .nd-input { flex: 1; }
.nd-path-input { cursor: default; padding-right: 36px; }
.nd-btn-secondary {
  display: inline-flex; align-items: center; gap: 4px;
  background: var(--hover-bg); border: 1px solid var(--border);
  border-radius: 8px; color: var(--text-dim); font-size: 11px; font-weight: 600;
  padding: 8px 10px; cursor: pointer; transition: all 0.12s ease;
  white-space: nowrap; flex-shrink: 0; height: 36px;
}
.nd-btn-secondary i { font-size: 14px; }
.nd-btn-secondary:hover { background: rgba(var(--accent-rgb), 0.08); color: var(--accent-strong); border-color: rgba(var(--accent-rgb), 0.2); }

.nd-category-error {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 8px 10px;
  background: rgba(239, 83, 80, 0.08);
  border: 1px solid rgba(239, 83, 80, 0.2);
  border-radius: 8px;
  font-size: 11px;
  color: var(--status-error);
}
.nd-category-error i { font-size: 15px; }

.nd-footer {
  position: relative; z-index: 1;
  display: flex; gap: 8px; padding: 10px 16px 14px;
  border-top: 1px solid var(--border); flex-shrink: 0;
}
.nd-btn {
  flex: 1; display: inline-flex; align-items: center; justify-content: center;
  gap: 6px; border: none; font-weight: 700; padding: 10px 16px;
  border-radius: 10px; cursor: pointer; font-size: 12.5px;
  transition: all 0.15s ease; font-family: inherit; min-width: 0;
}
.nd-btn i { font-size: 16px; }
.nd-btn:disabled, .nd-btn.disabled { opacity: 0.5; cursor: not-allowed; pointer-events: none; }
.nd-btn-primary {
  background: linear-gradient(135deg, var(--accent), rgba(var(--accent-rgb), 0.85));
  color: #0a0e1a;
  box-shadow: 0 2px 6px rgba(var(--accent-rgb), 0.2), inset 0 1px 0 rgba(255,255,255,0.15);
}
.nd-btn-primary:hover:not(.disabled) {
  box-shadow: 0 3px 12px rgba(var(--accent-rgb), 0.3), inset 0 1px 0 rgba(255,255,255,0.2);
  transform: translateY(-1px);
}
.nd-btn-primary:active:not(.disabled) { transform: scale(0.98); box-shadow: 0 1px 4px rgba(var(--accent-rgb), 0.15); }
.nd-btn-secondary {
  background: rgba(255,255,255,0.03); color: var(--text-muted);
  border: 1px solid var(--border);
}
.nd-btn-secondary:hover { background: rgba(var(--accent-rgb), 0.08); color: var(--accent-strong); border-color: rgba(var(--accent-rgb), 0.2); }
.nd-btn-secondary:active { transform: scale(0.98); }

.nd-duplicate-warning {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 12px 14px;
  background: rgba(255, 152, 0, 0.08);
  border: 1px solid rgba(255, 152, 0, 0.2);
  border-radius: 10px;
  margin-top: 8px;
}

.nd-duplicate-icon {
  width: 36px;
  height: 36px;
  border-radius: 8px;
  background: rgba(255, 152, 0, 0.15);
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
}

.nd-duplicate-icon i {
  font-size: 20px;
  color: #ff9800;
}

.nd-duplicate-info {
  flex: 1;
  min-width: 0;
}

.nd-duplicate-title {
  font-size: 13px;
  font-weight: 600;
  color: #ffb74d;
  display: block;
}

.nd-duplicate-detail {
  font-size: 11px;
  color: var(--text-dim);
  display: block;
  margin-top: 2px;
}

.nd-dupe-toggle {
  display: flex;
  align-items: center;
  gap: 6px;
  margin-top: 8px;
  background: none;
  border: none;
  padding: 0;
  cursor: pointer;
  color: var(--text);
  font-family: inherit;
}

.nd-dupe-track {
  width: 34px;
  height: 20px;
  border-radius: 10px;
  background: #50360f;
  border: 1px solid rgba(255, 152, 0, 0.25);
  position: relative;
  flex-shrink: 0;
  transition: background 0.15s ease, border-color 0.15s ease;
}

.nd-dupe-track.on {
  background: #c77f1a;
  border-color: #d99a2b;
}

.nd-dupe-thumb {
  position: absolute;
  top: 2px;
  left: 2px;
  width: 14px;
  height: 14px;
  border-radius: 50%;
  background: #a9791f;
  transition: transform 0.2s ease, background 0.15s ease;
}

.nd-dupe-track.on .nd-dupe-thumb {
  transform: translateX(14px);
  background: #ffb74d;
}

.nd-spinner { animation: ndSpin 0.7s linear infinite; }
@keyframes ndSpin { from { transform: rotate(0deg); } to { transform: rotate(360deg); } }

@media (max-width: 520px) {
  .nd-overlay { padding: 0; align-items: flex-end; }
  .nd-modal {
    max-width: 100%;
    border-radius: 20px 20px 0 0;
    animation: ndSlideUp 0.3s cubic-bezier(0.16, 1, 0.3, 1);
  }
  @keyframes ndSlideUp {
    from { opacity: 0; transform: translateY(100%); }
    to   { opacity: 1; transform: translateY(0); }
  }
  .nd-body { padding: 14px 16px 8px; }
  .nd-header { padding: 12px 12px 8px; }
  .nd-divider { margin: 0 12px; }
  .nd-footer { padding: 8px 12px 12px; }
}
</style>
