<template>
  <div v-if="visible" class="error-overlay">
    <div class="error-dialog">
      <div class="error-header">
        <h2 class="error-title">
          <i class="material-icons">warning</i> {{ t('Download Error') }}
        </h2>
        <button class="error-close" @click="$emit('close')">
          <i class="material-icons">close</i>
        </button>
      </div>
      <div class="error-section">
        <p class="error-label">{{ t('Filename:') }}</p>
        <p class="error-filename">{{ filename }}</p>
      </div>
      <div class="error-section">
        <p class="error-label">{{ t('Error Details:') }}</p>
        <pre class="error-message">{{ message }}</pre>
      </div>
      <div class="error-footer">
        <button class="error-btn" @click="$emit('close')">{{ t('Close') }}</button>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { useI18n } from '../i18n'

const { t } = useI18n()
defineProps<{
  visible: boolean
  filename: string
  message: string
}>()
defineEmits<{ close: [] }>()
</script>

<style scoped>
.error-overlay {
  position: fixed;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  background: rgba(0,0,0,0.7);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 1000;
}

.error-dialog {
  background: var(--bg-card);
  border-radius: 12px;
  padding: 32px;
  max-width: 600px;
  width: 90%;
  border: 1px solid var(--border);
}

.error-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 24px;
}

.error-title {
  color: var(--status-error);
  margin: 0;
  font-size: 20px;
  display: flex;
  align-items: center;
  gap: 8px;
}

.error-close {
  background: none;
  border: none;
  color: var(--text-dim);
  cursor: pointer;
  padding: 0 8px;
  display: flex;
  align-items: center;
  justify-content: center;
}

.error-section {
  margin-bottom: 20px;
}

.error-label {
  color: var(--text-dim);
  font-size: 12px;
  margin: 0 0 8px 0;
}

.error-filename {
  color: var(--text);
  font-weight: 500;
  margin: 0;
}

.error-message {
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: 6px;
  padding: 16px;
  color: var(--status-error);
  font-size: 13px;
  white-space: pre-wrap;
  word-break: break-word;
  max-height: 300px;
  overflow-y: auto;
  margin: 0;
}

.error-footer {
  display: flex;
  justify-content: flex-end;
}

.error-btn {
  background: var(--accent);
  border: none;
  color: #0a0e1a;
  font-weight: 600;
  padding: 10px 24px;
  border-radius: 8px;
  cursor: pointer;
  font-size: 14px;
}
</style>
