<template>
  <div class="pf-field pf-card" :class="{ enabled: model.enabled }">
    <div class="pf-row">
      <div class="pf-left">
        <i class="material-icons">web</i>
        <span>{{ t('Enable Proxy') }}</span>
      </div>
      <div class="pf-right">
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
          <label class="pf-sub">{{ t('Port') }}</label>
          <input :value="model.port" @change="setField('port', Number(($event.target as HTMLInputElement).value))" type="number" min="1" class="pf-input" placeholder="1080" />
        </div>
        <div class="pf-field pf-span-2">
          <label class="pf-sub">{{ t('Host') }}</label>
          <input :value="model.host" @input="setField('host', ($event.target as HTMLInputElement).value)" class="pf-input" :placeholder="t('proxy.example.com')" />
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
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from '../i18n'

export interface ProxyValue {
  enabled: boolean
  type: string
  host: string
  port: number
  user: string
  pass: string
}

const props = withDefaults(defineProps<{ modelValue?: ProxyValue }>(), {
  modelValue: () => ({ enabled: false, type: 'http', host: '', port: 1080, user: '', pass: '' }),
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
  background: var(--bg-surface);
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
.pf-input:focus { border-color: var(--accent-strong); }
.pf-input::placeholder { color: var(--text-dim); opacity: 0.5; }
.pf-input::-webkit-inner-spin-button,
.pf-input::-webkit-outer-spin-button { -webkit-appearance: none; margin: 0; }
.pf-input[type="number"] { -moz-appearance: textfield; appearance: textfield; }
</style>