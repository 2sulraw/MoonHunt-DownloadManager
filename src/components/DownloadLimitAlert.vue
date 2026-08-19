<!-- Download limit reached alert — two-step: acknowledge then optionally reset -->
<template>
  <Transition name="modal-fade">
    <div v-if="isOpen" class="dla-overlay" @mousedown.self.prevent="overlayPress()" @mouseup.self="overlayRelease() && handleKeepPaused()" @mouseleave="overlayCancel()">
      <Transition name="details-scale">
        <div v-if="isOpen" class="dla-modal" @click.stop>
          <!-- Step 1: Alert -->
          <template v-if="step === 'alert'">
            <div class="dla-icon-wrap">
              <i class="material-icons dla-icon">warning</i>
            </div>
            <h2 class="dla-title">{{ t('Download Limit Reached') }}</h2>
            <p class="dla-desc">
              {{ t("You've reached your configured download limit.") }}
            </p>
            <div class="dla-stats">
              <div class="dla-stat">
                <span class="dla-stat-label">{{ t('Downloaded') }}</span>
                <span class="dla-stat-value">{{ formatBytes(usedBytes) }}</span>
              </div>
              <div class="dla-stat-divider"></div>
              <div class="dla-stat">
                <span class="dla-stat-label">{{ t('Limit') }}</span>
                <span class="dla-stat-value accent">{{ formatBytes(limitBytes) }}</span>
              </div>
            </div>
            <div class="dla-progress-wrap">
              <div class="dla-progress-track">
                <div class="dla-progress-fill" :style="{ width: '100%' }"></div>
              </div>
            </div>
            <p class="dla-hint">{{ t('All active downloads have been paused.') }}</p>
            <div class="dla-footer">
              <button class="dla-btn dla-btn-primary" @click="handleAcknowledge">
                <i class="material-icons">check</i>
                {{ t('I Understand') }}
              </button>
            </div>
          </template>

          <!-- Step 2: Reset prompt -->
          <template v-else-if="step === 'reset'">
            <div class="dla-icon-wrap reset">
              <i class="material-icons dla-icon">restart_alt</i>
            </div>
            <h2 class="dla-title">{{ t('Reset Download Limit?') }}</h2>
            <p class="dla-desc">
              {{ t('Would you like to reset the download counter and continue downloading?') }}
            </p>
            <div class="dla-reset-info">
              <i class="material-icons">info_outline</i>
              <span>{{ t('Downloads will remain paused. You can resume them manually after resetting.') }}</span>
            </div>
            <div class="dla-footer dual">
              <button class="dla-btn dla-btn-ghost" @click="handleKeepPaused">
                <i class="material-icons">pause</i>
                {{ t('Keep Paused') }}
              </button>
              <button class="dla-btn dla-btn-primary" @click="handleReset">
                <i class="material-icons">restart_alt</i>
                {{ t('Reset & Continue') }}
              </button>
            </div>
          </template>
        </div>
      </Transition>
    </div>
  </Transition>
</template>

<script setup lang="ts">
import { ref } from 'vue'
import { formatBytes } from '../utils/format'
import { useOverlayClose } from '../composables/useOverlayClose'
import { useI18n } from '../i18n'

const { overlayPress, overlayCancel, overlayRelease } = useOverlayClose()
const { t } = useI18n()

const isOpen = ref(false)
const step = ref<'alert' | 'reset'>('alert')
const usedBytes = ref(0)
const limitBytes = ref(0)

let resolveFn: ((result: { reset: boolean }) => void) | null = null

function open(used: number, limit: number): Promise<{ reset: boolean }> {
  usedBytes.value = used
  limitBytes.value = limit
  step.value = 'alert'
  isOpen.value = true
  return new Promise((resolve) => {
    resolveFn = resolve
  })
}

function handleAcknowledge() {
  step.value = 'reset'
}

function handleReset() {
  isOpen.value = false
  resolveFn?.({ reset: true })
  resolveFn = null
}

function handleKeepPaused() {
  isOpen.value = false
  resolveFn?.({ reset: false })
  resolveFn = null
}

defineExpose({ open })
</script>

<style scoped>
.dla-overlay {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.75);
  backdrop-filter: blur(12px);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 9999;
}

.dla-modal {
  background: var(--bg-card);
  border: 1px solid var(--border-strong);
  border-radius: 16px;
  padding: 32px;
  max-width: 420px;
  width: 90%;
  text-align: center;
  box-shadow: var(--shadow-lg);
}

.dla-icon-wrap {
  width: 64px;
  height: 64px;
  border-radius: 50%;
  background: rgba(239, 83, 80, 0.12);
  border: 2px solid rgba(239, 83, 80, 0.25);
  display: flex;
  align-items: center;
  justify-content: center;
  margin: 0 auto 16px;
}

.dla-icon-wrap.reset {
  background: rgba(var(--accent-rgb), 0.12);
  border-color: rgba(var(--accent-rgb), 0.25);
}

.dla-icon {
  font-size: 32px;
  color: var(--status-error);
}

.dla-icon-wrap.reset .dla-icon {
  color: var(--accent-strong);
}

.dla-title {
  margin: 0 0 8px;
  font-size: 20px;
  font-weight: 700;
  color: var(--text);
  letter-spacing: -0.02em;
}

.dla-desc {
  margin: 0 0 20px;
  color: var(--text-muted);
  font-size: 14px;
  line-height: 1.5;
}

.dla-stats {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 24px;
  padding: 16px;
  background: var(--bg-surface);
  border: 1px solid var(--border);
  border-radius: 12px;
  margin-bottom: 16px;
}

.dla-stat {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 4px;
}

.dla-stat-label {
  font-size: 11px;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.05em;
  color: var(--text-dim);
}

.dla-stat-value {
  font-size: 22px;
  font-weight: 800;
  color: var(--text);
  letter-spacing: -0.02em;
}

.dla-stat-value.accent {
  color: var(--accent-strong);
}

.dla-stat-divider {
  width: 1px;
  height: 32px;
  background: var(--border-strong);
}

.dla-progress-wrap {
  margin-bottom: 12px;
}

.dla-progress-track {
  height: 6px;
  background: rgba(255, 255, 255, 0.06);
  border-radius: 3px;
  overflow: hidden;
}

.dla-progress-fill {
  height: 100%;
  background: #ef5350;
  border-radius: 3px;
}

.dla-hint {
  margin: 0 0 24px;
  color: var(--text-dim);
  font-size: 12px;
}

.dla-reset-info {
  display: flex;
  align-items: flex-start;
  gap: 8px;
  padding: 12px 14px;
  background: rgba(var(--accent-rgb), 0.06);
  border: 1px solid rgba(var(--accent-rgb), 0.12);
  border-radius: 8px;
  margin-bottom: 24px;
  text-align: left;
}

.dla-reset-info .material-icons {
  font-size: 16px;
  color: var(--accent-strong);
  flex-shrink: 0;
  margin-top: 1px;
}

.dla-reset-info span {
  color: var(--text-muted);
  font-size: 12.5px;
  line-height: 1.5;
}

.dla-footer {
  display: flex;
  justify-content: center;
}

.dla-footer.dual {
  gap: 10px;
}

.dla-btn {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 10px 20px;
  border-radius: 10px;
  font-size: 14px;
  font-weight: 600;
  cursor: pointer;
  border: none;
  transition: all 0.2s;
}

.dla-btn .material-icons {
  font-size: 18px;
}

.dla-btn-primary {
  background: #ef5350;
  color: #fff;
}

.dla-btn-primary:hover {
  background: #f44336;
  transform: translateY(-1px);
}

.dla-btn-ghost {
  background: var(--hover-bg);
  border: 1px solid var(--border-strong);
  color: var(--text-dim);
}

.dla-btn-ghost:hover {
  background: rgba(255, 255, 255, 0.08);
  color: var(--text);
}
</style>
