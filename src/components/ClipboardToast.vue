<template>
  <transition-group name="toast-slide" tag="div" class="clipboard-toast-container">
    <div v-for="toast in toasts" :key="toast.id" class="clipboard-toast">
      <div class="toast-body">
        <i class="material-icons toast-icon">link</i>
        <div class="toast-text">
          <span class="toast-title">{{ t('Download link detected') }}</span>
          <span class="toast-url">{{ toast.url }}</span>
        </div>
      </div>
      <div class="toast-actions">
        <button class="toast-btn toast-btn-primary" @click="dismiss(toast.id); $emit('add-download', toast.url)">{{ t('Add') }}</button>
        <button class="toast-btn toast-btn-ghost" @click="dismiss(toast.id)">{{ t('Dismiss') }}</button>
      </div>
    </div>
  </transition-group>
</template>

<script setup lang="ts">
import { ref } from 'vue'
import { useI18n } from '../i18n'

const { t } = useI18n()

interface Toast {
  id: number
  url: string
}

defineEmits<{ 'add-download': [url: string] }>()

const toasts = ref<Toast[]>([])
let nextId = 0

function show(url: string) {
  const id = nextId++
  toasts.value.push({ id, url })
  setTimeout(() => dismiss(id), 10000)
}

function dismiss(id: number) {
  toasts.value = toasts.value.filter(t => t.id !== id)
}

defineExpose({ show })
</script>

<style scoped>
.clipboard-toast-container {
  position: fixed;
  bottom: 100px;
  left: 50%;
  transform: translateX(-50%);
  display: flex;
  flex-direction: column-reverse;
  gap: 8px;
  z-index: 9999;
  pointer-events: none;
}
.clipboard-toast {
  pointer-events: auto;
  background: var(--bg-card);
  border: 1px solid var(--border-strong);
  border-radius: 12px;
  box-shadow: 0 8px 24px rgba(0, 0, 0, 0.2);
  padding: 12px 16px;
  display: flex;
  flex-direction: column;
  gap: 10px;
  width: 380px;
}
.toast-body {
  display: flex;
  align-items: flex-start;
  gap: 10px;
}
.toast-icon { font-size: 20px; color: var(--accent-strong); margin-top: 2px; }
.toast-text { display: flex; flex-direction: column; gap: 2px; min-width: 0; }
.toast-title { font-size: 12px; font-weight: 600; color: var(--text); }
.toast-url { font-size: 11px; color: var(--text-dim); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
.toast-actions { display: flex; gap: 6px; justify-content: flex-end; }
.toast-btn {
  padding: 5px 14px;
  border-radius: 6px;
  font-size: 12px;
  font-weight: 600;
  cursor: pointer;
  transition: all 0.15s;
  border: none;
}
.toast-btn-primary { background: var(--accent); color: #0a0e1a; }
.toast-btn-primary:hover { box-shadow: 0 2px 8px rgba(var(--accent-rgb), 0.3); }
.toast-btn-ghost { background: var(--hover-bg); color: var(--text); }
.toast-btn-ghost:hover { background: rgba(255, 255, 255, 0.08); }
.toast-slide-enter-active { transition: all 0.25s ease; }
.toast-slide-leave-active { transition: all 0.2s ease; }
.toast-slide-enter-from { opacity: 0; transform: translateY(20px); }
.toast-slide-leave-to { opacity: 0; transform: translateY(-10px); }
</style>
