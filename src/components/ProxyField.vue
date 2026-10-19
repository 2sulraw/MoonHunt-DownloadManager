<template>
  <div class="pf-field pf-card" :class="{ enabled: model.enabled }">
    <div class="pf-row">
      <div class="pf-left">
        <i class="material-icons">web</i>
        <span>{{ t('Enable Proxy') }}</span>
      </div>
      <div class="pf-right">
        <button
          v-if="detectable"
          type="button"
          class="pf-detect-btn"
          :disabled="detecting"
          @click="autoDetect"
          :title="t('Auto Detect')"
        >
          <i class="material-icons">{{ detecting ? 'hourglass_empty' : 'wifi_find' }}</i>
        </button>
        <div
          class="pf-toggle"
          :class="{ active: model.enabled }"
          role="switch"
          :aria-checked="model.enabled"
          @click="toggle"
        >
          <span class="pf-toggle-track"><span class="pf-toggle-thumb"></span></span>
        </div>
      </div>
    </div>

    <div v-if="model.enabled" class="pf-controls">
      <div class="pf-grid">
        <div class="pf-field">
          <label class="pf-sub">{{ t('Host') }}</label>
          <input :value="model.host" @input="setField('host', ($event.target as HTMLInputElement).value)" class="pf-input" :placeholder="t('proxy.example.com')" />
        </div>
        <div class="pf-field">
          <label class="pf-sub">{{ t('Port') }}</label>
          <input :value="model.port" @change="setField('port', Number(($event.target as HTMLInputElement).value))" type="number" min="1" class="pf-input" placeholder="1080" />
        </div>
        <div class="pf-field">
          <label class="pf-sub">{{ t('Username') }} <span class="pf-opt">{{ t('(optional)') }}</span></label>
          <input :value="model.user" @input="setField('user', ($event.target as HTMLInputElement).value)" class="pf-input" :placeholder="t('Leave blank if none')" />
        </div>
        <div class="pf-field">
          <label class="pf-sub">{{ t('Password') }} <span class="pf-opt">{{ t('(optional)') }}</span></label>
          <input :value="model.pass" @input="setField('pass', ($event.target as HTMLInputElement).value)" type="password" class="pf-input" :placeholder="t('Leave blank if none')" />
        </div>
      </div>
    </div>
    <div v-if="detectMsg" class="pf-detect-msg" :class="{ ok: detectOk }">{{ detectMsg }}</div>
    <div v-if="$slots.actions" class="pf-actions">
      <slot name="actions" />
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { useI18n } from '../i18n'

export interface ProxyValue {
  enabled: boolean
  type: string
  host: string
  port: number
  user: string
  pass: string
}

const props = withDefaults(defineProps<{ modelValue?: ProxyValue; detectable?: boolean }>(), {
  modelValue: () => ({ enabled: false, type: 'http', host: '', port: 1080, user: '', pass: '' }),
  detectable: true,
})
const emit = defineEmits<{
  (e: 'update:modelValue', value: ProxyValue): void
}>()
const { t } = useI18n()

const model = computed(() => props.modelValue)

function toggle() {
  emit('update:modelValue', { ...props.modelValue, enabled: !props.modelValue.enabled })
}

function setField<K extends keyof ProxyValue>(key: K, value: ProxyValue[K]) {
  emit('update:modelValue', { ...props.modelValue, [key]: value })
}

const detecting = ref(false)
const detectMsg = ref('')
const detectOk = ref(false)

async function autoDetect() {
  if (detecting.value) return
  detecting.value = true
  detectMsg.value = ''
  try {
    const result = await invoke<{ enabled: boolean; host: string; port: number; message: string }>('detect_system_proxy')
    detectOk.value = !!result.enabled
    detectMsg.value = result.message || ''
    if (result.enabled && result.host && result.port) {
      emit('update:modelValue', { ...props.modelValue, enabled: true, type: 'http', host: result.host, port: result.port })
    }
  } catch (e: unknown) {
    detectOk.value = false
    detectMsg.value = e instanceof Error ? e.message : String(e)
  } finally {
    detecting.value = false
  }
}
</script>

<style scoped>
.pf-card {
  background: rgba(255, 255, 255, 0.02);
  border: 1px solid var(--border);
  border-radius: 12px;
  padding: 12px 14px;
  transition: all 0.25s ease;
}

.pf-card.enabled {
  background: rgba(var(--accent-rgb), 0.04);
  border-color: rgba(var(--accent-rgb), 0.18);
}

.pf-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 10px;
}

.pf-left {
  display: flex;
  align-items: center;
  gap: 8px;
  color: var(--text-muted);
  font-size: 13px;
}

.pf-left .material-icons {
  font-size: 18px;
  color: var(--text-dim);
}

.pf-card.enabled .pf-left {
  color: var(--text);
}

.pf-card.enabled .pf-left .material-icons {
  color: var(--accent-strong);
}

.pf-right {
  display: flex;
  align-items: center;
  gap: 8px;
}

.pf-detect-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 30px;
  height: 30px;
  border-radius: 8px;
  border: 1px solid transparent;
  background: transparent;
  color: var(--text-dim);
  cursor: pointer;
  padding: 0;
  transition: all 0.15s;
}
.pf-detect-btn i { font-size: 17px; }
.pf-detect-btn:hover:not(:disabled) { color: var(--accent-strong); border-color: var(--border); background: var(--hover-bg); }
.pf-detect-btn:disabled { opacity: 0.5; cursor: not-allowed; }

.pf-detect-msg { margin-top: 8px; font-size: 11.5px; color: var(--text-dim); }
.pf-detect-msg.ok { color: var(--status-complete); }

.pf-actions { margin-top: 11px; padding-top: 11px; border-top: 1px solid var(--border); }

.pf-toggle {
  width: 42px;
  height: 24px;
  border-radius: 12px;
  background: var(--hover-bg);
  border: 1px solid var(--border-strong);
  cursor: pointer;
  position: relative;
  flex-shrink: 0;
  transition: background 0.25s ease, border-color 0.25s ease;
}

.pf-toggle.active {
  background: rgba(var(--accent-rgb), 0.25);
  border-color: rgba(var(--accent-rgb), 0.4);
}

.pf-toggle-track {
  position: absolute;
  top: 2px;
  left: 2px;
  width: 18px;
  height: 18px;
  border-radius: 50%;
  transition: transform 0.25s cubic-bezier(0.34, 1.4, 0.64, 1);
}

.pf-toggle.active .pf-toggle-track {
  transform: translateX(18px);
}

.pf-toggle-thumb {
  position: absolute;
  inset: 0;
  border-radius: 50%;
  background: var(--text-dim);
  transition: background 0.25s ease;
}

.pf-toggle.active .pf-toggle-thumb {
  background: var(--accent);
}

.pf-controls {
  margin-top: 11px;
  padding-top: 11px;
  border-top: 1px solid var(--border);
}

.pf-grid {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 8px;
}

.pf-span-2 { grid-column: span 2; }
.pf-field { display: flex; flex-direction: column; gap: 6px; }
.pf-sub {
  font-size: 11px;
  font-weight: 500;
  color: var(--text-dim);
}
.pf-opt { font-weight: 400; color: var(--text-dim); font-size: 10px; }
.pf-input {
  width: 100%;
  height: 32px;
  padding: 0 10px;
  background: #1b1b2f;
  border: 1px solid var(--border);
  border-radius: 8px;
  color: var(--text);
  font-size: 12px;
  font-family: inherit;
  outline: none;
  transition: all 0.15s ease;
  box-sizing: border-box;
}
.pf-input:hover { border-color: var(--border-strong); }
.pf-input:focus { background: #191b2d; border-color: var(--accent-strong); }
.pf-input::placeholder { color: var(--text-dim); opacity: 0.5; }
.pf-input::-webkit-inner-spin-button,
.pf-input::-webkit-outer-spin-button { -webkit-appearance: none; margin: 0; }
.pf-input[type="number"] { -moz-appearance: textfield; appearance: textfield; }
</style>