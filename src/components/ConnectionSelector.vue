<template>
  <div class="connection-selector">
    <div class="cs-stepper" role="group" :aria-label="t('Connections')">
      <button
        class="cs-step"
        type="button"
        @click="update(modelValue - 1)"
        :disabled="modelValue <= 1"
        :aria-label="t('Decrease connections')"
      >
        <i class="material-icons">remove</i>
      </button>
      <input
        :value="modelValue"
        @input="onInput"
        @keydown.enter="$emit('enter')"
        type="number"
        min="1"
        class="cs-input"
        :aria-label="t('Connections')"
        aria-valuemin="1"
        :aria-valuenow="modelValue"
      />
      <button
        class="cs-step"
        type="button"
        @click="update(modelValue + 1)"
        :aria-label="t('Increase connections')"
      >
        <i class="material-icons">add</i>
      </button>
    </div>
  </div>
</template>

<script setup lang="ts">
import { useI18n } from '../i18n'

const props = withDefaults(defineProps<{ modelValue: number }>(), { modelValue: 1 })
const emit = defineEmits<{
  (e: 'update:modelValue', value: number): void
  (e: 'enter'): void
}>()
const { t } = useI18n()

function update(value: number) {
  const next = Math.max(1, Math.round(value) || 1)
  emit('update:modelValue', next)
}

function onInput(e: Event) {
  const target = e.target as HTMLInputElement
  const next = Number(target.value)
  if (Number.isFinite(next)) update(next)
}
</script>

<style scoped>
.connection-selector {
  display: inline-flex;
}

.cs-stepper {
  display: flex;
  align-items: center;
  height: 32px;
  border: 1px solid var(--border-strong);
  border-radius: 9px;
  background: var(--bg-surface);
  overflow: hidden;
  flex-shrink: 0;
}

.cs-step {
  width: 30px;
  height: 100%;
  border: none;
  background: var(--bg-card-alt);
  color: var(--text-muted);
  display: flex;
  align-items: center;
  justify-content: center;
  cursor: pointer;
  transition: background 0.12s, color 0.12s;
  padding: 0;
}

.cs-step:hover:not(:disabled) { background: var(--hover-bg); color: var(--text); }
.cs-step:disabled { opacity: 0.3; cursor: not-allowed; }
.cs-step i {
  font-size: 16px;
  color: inherit;
}

html.linear-dark .cs-step {
  background: #18191a;
  color: #62666d;
}

html.linear-dark .cs-step:hover:not(:disabled) {
  background: var(--hover-bg);
  color: var(--text);
}

.cs-input {
  width: 44px;
  height: 100%;
  border: none;
  border-left: 1px solid var(--border);
  border-right: 1px solid var(--border);
  background: transparent;
  text-align: center;
  color: var(--text);
  font-weight: 700;
  font-size: 12px;
  font-family: inherit;
  outline: none;
  -moz-appearance: textfield;
}

.cs-input::-webkit-outer-spin-button,
.cs-input::-webkit-inner-spin-button { -webkit-appearance: none; margin: 0; }
</style>
