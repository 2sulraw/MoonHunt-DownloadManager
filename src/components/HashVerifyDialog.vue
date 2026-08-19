<template>
  <transition name="modal-fade">
    <div v-if="visible" class="hash-overlay" @mousedown.self.prevent="overlayPress()" @mouseup.self="overlayRelease() && $emit('close')" @mouseleave="overlayCancel()">
      <div class="hash-dialog">
        <div class="hash-header">
          <div class="hash-header-icon">
            <i class="material-icons">verified_user</i>
          </div>
          <div>
            <h2 class="hash-title">{{ t('Verify File Hash') }}</h2>
            <p class="hash-subtitle">{{ filename }}</p>
          </div>
        </div>

        <div class="hash-field">
          <label class="hash-label">{{ t('Algorithm') }}</label>
          <div class="hash-algo-row">
            <button v-for="a in algorithms" :key="a" @click="selectedAlgo = a" class="hash-algo-btn" :class="{ active: selectedAlgo === a }">
              {{ a.toUpperCase() }}
            </button>
          </div>
        </div>

        <div class="hash-field">
          <label class="hash-label">{{ t('Expected Hash') }} <span class="hash-optional">{{ t('(paste if you have one)') }}</span></label>
          <input v-model="expectedHash" class="hash-input" :placeholder="t('Paste expected hash here...')" />
        </div>

        <div v-if="computedHash" class="hash-result" :class="matchResult === true ? 'match' : matchResult === false ? 'mismatch' : 'pending'">
          <div class="hash-result-icon">
            <i class="material-icons">{{ matchResult === true ? 'check_circle' : matchResult === false ? 'error' : 'info' }}</i>
          </div>
          <div class="hash-result-text">
            <span v-if="matchResult === true" class="hash-match">{{ t('Hash matches!') }}</span>
            <span v-else-if="matchResult === false" class="hash-mismatch">{{ t('Hash mismatch!') }}</span>
            <span v-else>{{ t('Computing...') }}</span>
            <code v-if="matchResult !== null" class="hash-value">{{ computedHash }}</code>
          </div>
        </div>

        <div v-if="error" class="hash-error">
          <i class="material-icons">warning</i>
          <span>{{ error }}</span>
        </div>

        <div class="hash-footer">
          <button @click="$emit('close')" class="hash-btn hash-btn-ghost">{{ t('Cancel') }}</button>
          <button @click="computeHash" :disabled="isComputing || !filePath" class="hash-btn hash-btn-primary">
            <i v-if="isComputing" class="material-icons hash-spinner">hourglass_top</i>
            {{ isComputing ? t('Computing...') : t('Verify Hash') }}
          </button>
        </div>
      </div>
    </div>
  </transition>
</template>

<script setup lang="ts">
import { ref, computed, watch } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { useOverlayClose } from '../composables/useOverlayClose'
import { useI18n } from '../i18n'

const props = defineProps<{
  visible: boolean
  filePath: string
  filename: string
}>()

defineEmits<{ close: [] }>()

const { overlayPress, overlayCancel, overlayRelease } = useOverlayClose()
const { t } = useI18n()

const algorithms = ['md5', 'sha1', 'sha256', 'sha512']
const selectedAlgo = ref('sha256')
const expectedHash = ref('')
const computedHash = ref('')
const isComputing = ref(false)
const error = ref('')

const matchResult = computed(() => {
  if (!computedHash.value || !expectedHash.value.trim()) return null
  return computedHash.value.toLowerCase() === expectedHash.value.trim().toLowerCase()
})

watch(() => props.visible, (v) => {
  if (!v) {
    computedHash.value = ''
    expectedHash.value = ''
    error.value = ''
    isComputing.value = false
  }
})

async function computeHash() {
  if (!props.filePath || isComputing.value) return
  isComputing.value = true
  error.value = ''
  computedHash.value = ''
  try {
    computedHash.value = await invoke<string>('compute_file_hash', {
      filePath: props.filePath,
      algorithm: selectedAlgo.value,
    })
  } catch (e: unknown) {
    error.value = e instanceof Error ? e.message : String(e)
  } finally {
    isComputing.value = false
  }
}
</script>

<style scoped>
.hash-overlay {
  position: fixed; inset: 0; background: rgba(0,0,0,0.55); backdrop-filter: blur(8px);
  display: flex; align-items: center; justify-content: center; z-index: 9999;
}
.hash-dialog {
  background: var(--bg-card); border: 1px solid var(--border-strong); border-radius: 16px;
  width: min(440px, 90vw); padding: 24px; display: flex; flex-direction: column; gap: 16px;
  box-shadow: 0 16px 48px rgba(0,0,0,0.25);
}
.hash-header { display: flex; align-items: center; gap: 12px; }
.hash-header-icon {
  width: 40px; height: 40px; border-radius: 10px;
  background: rgba(var(--accent-rgb), 0.1); display: flex; align-items: center; justify-content: center;
}
.hash-header-icon i { font-size: 20px; color: var(--accent-strong); }
.hash-title { margin: 0; font-size: 16px; font-weight: 700; color: var(--text); }
.hash-subtitle { margin: 2px 0 0; font-size: 12px; color: var(--text-dim); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; max-width: 320px; }
.hash-field { display: flex; flex-direction: column; gap: 6px; }
.hash-label { font-size: 12px; font-weight: 600; color: var(--text-muted); }
.hash-optional { font-weight: 400; color: var(--text-dim); font-size: 11px; }
.hash-algo-row { display: flex; gap: 6px; }
.hash-algo-btn {
  padding: 6px 14px; border-radius: 8px; border: 1px solid var(--border);
  background: var(--hover-bg); color: var(--text-dim); font-size: 12px; font-weight: 600;
  cursor: pointer; transition: all 0.15s;
}
.hash-algo-btn:hover { border-color: rgba(var(--accent-rgb), 0.2); }
.hash-algo-btn.active { background: rgba(var(--accent-rgb), 0.1); color: var(--accent-strong); border-color: rgba(var(--accent-rgb), 0.3); }
.hash-input {
  width: 100%; height: 38px; background: rgba(255,255,255,0.03); border: 1px solid var(--border);
  border-radius: 8px; padding: 0 12px; color: var(--text); font-size: 12px; font-family: monospace;
  outline: none; box-sizing: border-box; transition: border-color 0.15s;
}
.hash-input:focus { border-color: var(--accent-strong); background: rgba(var(--accent-rgb), 0.03); }
.hash-input::placeholder { color: var(--text-dim); font-family: inherit; }
.hash-result {
  display: flex; align-items: center; gap: 10px; padding: 10px 12px; border-radius: 10px;
  border: 1px solid var(--border);
}
.hash-result.match { background: rgba(76, 175, 80, 0.08); border-color: rgba(76, 175, 80, 0.25); }
.hash-result.mismatch { background: rgba(239, 83, 80, 0.08); border-color: rgba(239, 83, 80, 0.25); }
.hash-result-icon i { font-size: 20px; }
.hash-result.match .hash-result-icon i { color: #4caf50; }
.hash-result.mismatch .hash-result-icon i { color: var(--status-error); }
.hash-result-text { display: flex; flex-direction: column; gap: 2px; min-width: 0; }
.hash-match { color: #4caf50; font-size: 13px; font-weight: 600; }
.hash-mismatch { color: var(--status-error); font-size: 13px; font-weight: 600; }
.hash-value { font-size: 11px; color: var(--text-dim); word-break: break-all; font-family: monospace; }
.hash-error { display: flex; align-items: center; gap: 6px; padding: 8px 12px; background: rgba(239,83,80,0.08); border-radius: 8px; color: var(--status-error); font-size: 12px; }
.hash-error i { font-size: 16px; }
.hash-footer { display: flex; gap: 8px; justify-content: flex-end; padding-top: 4px; }
.hash-btn {
  display: inline-flex; align-items: center; gap: 6px; padding: 8px 18px; border-radius: 8px;
  font-size: 13px; font-weight: 600; cursor: pointer; transition: all 0.15s; border: none;
}
.hash-btn-ghost { background: var(--hover-bg); color: var(--text); }
.hash-btn-ghost:hover { background: rgba(255,255,255,0.08); }
.hash-btn-primary { background: var(--accent); color: #0a0e1a; }
.hash-btn-primary:hover { box-shadow: 0 2px 8px rgba(var(--accent-rgb), 0.3); }
.hash-btn-primary:disabled { opacity: 0.5; cursor: not-allowed; }
@keyframes spin { from { transform: rotate(0deg); } to { transform: rotate(360deg); } }
.hash-spinner { animation: spin 1s linear infinite; font-size: 16px; }
</style>
