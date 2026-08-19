<template>
  <transition name="modal-fade">
    <div v-if="visible" class="nd-overlay" @mousedown.self.prevent="overlayPress()" @mouseup.self="overlayRelease() && $emit('close')" @mouseleave="overlayCancel()">
      <transition name="nd-slide">
        <div class="nd-modal batch-nd-modal" @click.stop>
          <div class="nd-glow"></div>

          <header class="nd-header">
            <div class="nd-header-left">
              <div class="nd-avatar">
                <i class="material-icons">playlist_add</i>
              </div>
              <div class="nd-titles">
                <h1 class="nd-title">{{ t('Batch Download') }}</h1>
                <span class="nd-subtitle">{{ t('Paste multiple URLs, one per line. Supports templates like [1-100] and {mp4,webm}') }}</span>
              </div>
            </div>
            <div class="nd-header-actions">
              <button class="nd-icon-btn nd-close" @click="$emit('close')" :aria-label="t('Close')" :title="t('Close (Esc)')">
                <i class="material-icons">close</i>
              </button>
            </div>
          </header>

          <div class="nd-divider"></div>

          <div class="nd-body">
            <div class="nd-section">
              <div class="nd-section-header">
                <h3 class="nd-section-title">{{ t('URLs') }}</h3>
                <span v-if="expandedCount > 0" class="nd-source-badge">{{ expandedCount }} {{ t('URLs') }}</span>
              </div>
              <textarea
                v-model="urlText"
                class="batch-textarea"
                rows="7"
                placeholder="https://example.com/file001.zip&#10;https://example.com/file[002-010].zip&#10;https://example.com/video.[mp4,webm]"
              ></textarea>
            </div>

            <div v-if="expandedCount > 0" class="batch-preview">
              <div class="batch-preview-header">
                <i class="material-icons">visibility</i>
                <span>{{ t('Preview (first 10)') }}</span>
              </div>
              <div class="batch-preview-list">
                <div v-for="(u, i) in expandedPreview" :key="i" class="batch-preview-item">{{ u }}</div>
                <div v-if="expandedCount > 10" class="batch-preview-more">... {{ t('and') }} {{ expandedCount - 10 }} {{ t('more') }}</div>
              </div>
            </div>

            <div class="batch-row">
              <div class="nd-field nd-field-grow">
                <label class="nd-label">
                  <i class="material-icons nd-label-icon">folder</i>
                  {{ t('Save Folder') }} <span class="nd-optional">{{ t('(optional)') }}</span>
                </label>
                <input v-model="dir" class="nd-input" placeholder="C:\Downloads" />
              </div>
              <div class="nd-field batch-connections-field">
                <label class="nd-label">
                  <i class="material-icons nd-label-icon">layers</i>
                  {{ t('Connections') }}
                </label>
                <input v-model.number="connections" type="number" min="1" class="nd-input" />
              </div>
            </div>

            <div v-if="error" class="nd-category-error">
              <i class="material-icons">warning</i>
              <span>{{ error }}</span>
            </div>
          </div>

          <footer class="nd-footer">
            <button @click="$emit('close')" class="nd-btn nd-btn-secondary">
              <i class="material-icons">close</i> {{ t('Cancel') }}
            </button>
            <button @click="submit" :disabled="isSubmitting || expandedCount === 0" class="nd-btn nd-btn-primary" :class="{ disabled: isSubmitting || expandedCount === 0 }">
              <i v-if="isSubmitting" class="material-icons nd-spinner">hourglass_top</i>
              <i v-else class="material-icons">download</i>
              <span>{{ isSubmitting ? t('Adding...') : t('Add') + ' ' + expandedCount + ' ' + t('Downloads') }}</span>
            </button>
          </footer>
        </div>
      </transition>
    </div>
  </transition>
</template>

<script setup lang="ts">
import { ref, computed, watch, inject } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { expandTemplate } from '../utils/batchExpand'
import { useOverlayClose } from '../composables/useOverlayClose'
import { useI18n } from '../i18n'

const props = defineProps<{ visible: boolean }>()
const emit = defineEmits<{ close: [] }>()

const { overlayPress, overlayCancel, overlayRelease } = useOverlayClose()
const { t } = useI18n()
const rememberRequestedConnections = inject<(url: string, connections: number) => void>('rememberRequestedConnections', () => {})

const urlText = ref('')
const dir = ref('')
const connections = ref(1)
const isSubmitting = ref(false)
const error = ref('')

const expandedUrls = computed(() => {
  return urlText.value.split('\n').flatMap(line => expandTemplate(line.trim()))
})

const expandedCount = computed(() => expandedUrls.value.length)
const expandedPreview = computed(() => expandedUrls.value.slice(0, 10))

watch(() => props.visible, (v) => {
  if (!v) { urlText.value = ''; dir.value = ''; connections.value = 1; error.value = ''; isSubmitting.value = false }
})

async function submit() {
  if (expandedCount.value === 0 || isSubmitting.value) return
  isSubmitting.value = true
  error.value = ''
  try {
    const connectionCount = Math.max(1, Number(connections.value) || 1)
    const promises = expandedUrls.value.map(url => {
      rememberRequestedConnections(url, connectionCount)
      return invoke('add_download', {
        url,
        filename: null,
        dir: dir.value || null,
        connections: connectionCount,
      })
    })
    await Promise.all(promises)
    emit('close')
  } catch (e: unknown) {
    error.value = e instanceof Error ? e.message : String(e)
  } finally {
    isSubmitting.value = false
  }
}
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

.nd-field { display: flex; flex-direction: column; gap: 7px; }
.nd-field-grow { flex: 1; min-width: 0; }
.nd-label {
  display: flex; align-items: center; gap: 5px;
  font-size: 12px; font-weight: 600; color: var(--text-muted);
  letter-spacing: 0.01em;
}
.nd-label-icon { font-size: 14px; color: var(--text-muted); opacity: 0.7; }
.nd-optional { font-weight: 400; color: var(--text-dim); font-size: 11px; }

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

.batch-textarea {
  width: 100%; min-height: 120px;
  background: rgba(255,255,255,0.025);
  border: 1px solid var(--border);
  border-radius: 8px;
  padding: 10px 12px;
  color: var(--text);
  font-size: 12px;
  font-family: monospace;
  outline: none;
  resize: vertical;
  box-sizing: border-box;
  transition: all 0.15s ease;
}
.batch-textarea:hover { border-color: rgba(255,255,255,0.1); background: rgba(255,255,255,0.035); }
.batch-textarea:focus { border-color: var(--accent-strong); background: rgba(var(--accent-rgb), 0.03); }
.batch-textarea::placeholder { color: var(--text-dim); opacity: 0.55; font-family: inherit; }

.batch-row { display: flex; gap: 12px; }
.batch-connections-field { width: 92px; flex-shrink: 0; }

.batch-preview {
  background: rgba(255,255,255,0.02);
  border: 1px solid var(--border);
  border-radius: 10px;
  padding: 10px 12px;
  max-height: 150px;
  overflow-y: auto;
}
.batch-preview-header {
  display: flex; align-items: center; gap: 6px;
  font-size: 11px; font-weight: 600; color: var(--text-muted);
  margin-bottom: 6px;
}
.batch-preview-header i { font-size: 14px; color: var(--accent-strong); }
.batch-preview-list { display: flex; flex-direction: column; gap: 2px; }
.batch-preview-item {
  font-size: 11px; color: var(--text); font-family: monospace;
  white-space: nowrap; overflow: hidden; text-overflow: ellipsis;
}
.batch-preview-more { font-size: 11px; color: var(--text-dim); font-style: italic; }

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
.nd-btn:disabled { opacity: 0.5; cursor: not-allowed; }
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
    to { opacity: 1; transform: translateY(0); }
  }
  .nd-body { padding: 14px 16px 8px; }
  .nd-header { padding: 12px 12px 8px; }
  .nd-divider { margin: 0 12px; }
  .nd-footer { padding: 8px 12px 12px; }
}
</style>
