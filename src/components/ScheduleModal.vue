<!-- Compact modal for configuring download schedules -->
<template>
  <transition name="sm-fade">
    <div v-if="isOpen" class="sm-overlay" @mousedown.self.prevent="overlayPress()" @mouseup.self="overlayRelease() && cancel()" @mouseleave="overlayCancel()">
      <div class="sm-panel" ref="panelRef">
        <!-- Header -->
        <div class="sm-head">
          <div class="sm-head-left">
            <i class="material-icons sm-head-icon">event</i>
            <span class="sm-head-title">{{ t('Schedule') }}</span>
          </div>
          <button class="sm-close" @click="cancel">
            <i class="material-icons">close</i>
          </button>
        </div>

        <div class="sm-divider" />

        <!-- Body -->
        <div class="sm-body">
          <!-- Action -->
          <div class="sm-field">
            <span class="sm-label">{{ t('Action') }}</span>
            <div class="sm-segmented">
              <span
                class="sm-seg-indicator"
                :class="{ pause: action === 'pause', resume: action === 'resume' }"
              ></span>
              <button
                v-for="opt in actionOptions"
                :key="opt.value"
                :class="['sm-seg-btn', { active: action === opt.value }]"
                @click="action = opt.value"
              >{{ opt.label }}</button>
            </div>
          </div>

          <!-- Date & Time -->
          <div class="sm-field">
            <span class="sm-label">{{ t('Date & Time') }}</span>
            <n-date-picker
              v-model:value="time"
              type="datetime"
              format="yyyy-MM-dd hh:mm a"
              :is-date-disabled="isDateDisabled"
              :show-second="false"
              :placeholder="t('Pick date & time...')"
              size="small"
              style="width: 100%"
            />
          </div>

          <!-- Repeat -->
          <div class="sm-field">
            <span class="sm-label">{{ t('Repeat') }}</span>
            <div class="sm-segmented">
              <button
                v-for="opt in repeatOptions"
                :key="opt.value"
                :class="['sm-seg-btn', { active: repeat === opt.value }]"
                @click="repeat = opt.value"
              >{{ opt.label }}</button>
            </div>
          </div>
        </div>

        <div class="sm-divider" />
        <div class="sm-foot">
          <button class="sm-btn sm-btn-ghost" @click="cancel">{{ t('Cancel') }}</button>
          <button class="sm-btn sm-btn-accent" :disabled="!time" @click="confirm">
            <i class="material-icons sm-btn-ico">check</i>
            {{ t('Set Schedule') }}
          </button>
        </div>
      </div>
    </div>
  </transition>
</template>

<script setup lang="ts">
import { ref, provide } from 'vue'
import { useOverlayClose } from '../composables/useOverlayClose'
import { useI18n } from '../i18n'

const { overlayPress, overlayCancel, overlayRelease } = useOverlayClose()
const { t } = useI18n()

// naive-ui's useAdjustedTo checks for 'n-modal-body' injection key.
const modalBodyKey = 'n-modal-body' as const

const isOpen = ref(false)
const action = ref('start')
const time = ref<number | null>(null)
const repeat = ref('once')

// Tell naive-ui components (like NDatePicker) to teleport inside this modal
const panelRef = ref<HTMLElement | null>(null)
provide(modalBodyKey, panelRef)

const actionOptions = [
  { value: 'start', label: t('Start') },
  { value: 'pause', label: t('Pause') },
  { value: 'resume', label: t('Resume') },
]

const repeatOptions = [
  { value: 'once', label: t('Once') },
  { value: 'hourly', label: t('Hourly') },
  { value: 'daily', label: t('Daily') },
  { value: 'weekly', label: t('Weekly') },
]

export interface ScheduleResult {
  action: string
  scheduledTime: number
  repeatInterval: number
  enabled: boolean
  createdAt: number
}

let resolveFn: ((result: ScheduleResult) => void) | null = null

function open() {
  action.value = 'start'
  time.value = null
  repeat.value = 'once'
  isOpen.value = true
  return new Promise<ScheduleResult>((resolve) => {
    resolveFn = resolve
  })
}

function confirm() {
  if (!time.value) return
  const scheduledTs = time.value
  if (scheduledTs <= Date.now()) return // reject past times
  resolveFn?.({
    action: action.value,
    scheduledTime: scheduledTs,
    repeatInterval: getRepeatMs(repeat.value),
    enabled: true,
    createdAt: Date.now(),
  })
  isOpen.value = false
}

function cancel() {
  resolveFn?.(null as any)
  isOpen.value = false
}

//// Disable past dates in the date picker
function isDateDisabled(ts: number) {
  const today = new Date()
  today.setHours(0, 0, 0, 0)
  return ts < today.getTime()
}

//// Convert repeat interval string to milliseconds.
function getRepeatMs(r: string): number {
  switch (r) {
    case 'hourly': return 60 * 60 * 1000
    case 'daily': return 24 * 60 * 60 * 1000
    case 'weekly': return 7 * 24 * 60 * 60 * 1000
    default: return 0
  }
}

defineExpose({ open })
</script>

<style scoped>
.sm-overlay {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.55);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 10000;
  backdrop-filter: blur(6px);
}

.sm-panel {
  background: var(--bg-card);
  border: 1px solid var(--border-strong);
  border-radius: 14px;
  width: 320px;
  max-width: 92vw;
  max-height: 80vh;
  display: flex;
  flex-direction: column;
  box-shadow: var(--shadow-lg);
  animation: sm-pop 0.22s cubic-bezier(0.16, 1, 0.3, 1);
}

@keyframes sm-pop {
  from { opacity: 0; transform: scale(0.95) translateY(6px); }
  to   { opacity: 1; transform: scale(1) translateY(0); }
}

.sm-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 12px 14px;
}

.sm-head-left {
  display: flex;
  align-items: center;
  gap: 7px;
}

.sm-head-icon {
  font-size: 16px;
  color: var(--accent-strong);
}

.sm-head-title {
  font-size: 13px;
  font-weight: 700;
  color: var(--text);
}

.sm-close {
  width: 22px;
  height: 22px;
  border-radius: 5px;
  background: transparent;
  border: none;
  color: var(--text-dim);
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: background 0.12s, color 0.12s;
}

.sm-close:hover {
  background: var(--hover-bg);
  color: var(--text);
}

.sm-close .material-icons {
  font-size: 15px;
}

.sm-divider {
  height: 1px;
  background: var(--border);
}

.sm-body {
  padding: 12px 14px;
  display: flex;
  flex-direction: column;
  gap: 10px;
  overflow-y: auto;
}

.sm-field {
  display: flex;
  flex-direction: column;
  gap: 5px;
}

.sm-label {
  font-size: 10px;
  font-weight: 700;
  color: var(--text-dim);
  text-transform: uppercase;
  letter-spacing: 0.6px;
}

.sm-segmented {
  position: relative;
  display: flex;
  gap: 3px;
  background: var(--hover-bg);
  border-radius: 8px;
  padding: 3px;
}

.sm-seg-btn {
  position: relative;
  z-index: 1;
  flex: 1;
  height: 26px;
  border: none;
  border-radius: 6px;
  background: transparent;
  color: var(--text-dim);
  font-size: 11px;
  font-weight: 600;
  font-family: inherit;
  cursor: pointer;
  transition: background 0.12s, color 0.12s;
}

.sm-seg-btn:hover:not(.active) {
  color: var(--text-muted);
  background: rgba(255, 255, 255, 0.04);
}

.sm-seg-btn.active {
  background: transparent;
  color: var(--accent-strong);
  box-shadow: none;
}

.sm-seg-indicator {
  position: absolute;
  top: 3px;
  left: 3px;
  width: calc((100% - 12px) / 3);
  height: 26px;
  background: var(--bg-card);
  border-radius: 6px;
  box-shadow: 0 1px 3px rgba(0, 0, 0, 0.18), inset 0 0 0 1px var(--border);
  pointer-events: none;
  transition: transform 0.22s cubic-bezier(0.4, 0, 0.2, 1);
  z-index: 0;
}

.sm-seg-indicator.pause {
  transform: translateX(calc(100% + 3px));
}

.sm-seg-indicator.resume {
  transform: translateX(calc(200% + 6px));
}

.sm-foot {
  display: flex;
  gap: 6px;
  padding: 10px 14px 12px;
  justify-content: flex-end;
  flex-shrink: 0;
}

.sm-btn {
  height: 30px;
  border-radius: 7px;
  border: none;
  font-size: 11.5px;
  font-weight: 600;
  font-family: inherit;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 4px;
  transition: background 0.12s, color 0.12s, opacity 0.12s;
  padding: 0 14px;
}

.sm-btn-ghost {
  background: transparent;
  color: var(--text-dim);
}

.sm-btn-ghost:hover {
  background: var(--hover-bg);
  color: var(--text-muted);
}

.sm-btn-accent {
  background: var(--accent);
  color: #0a0e1a;
}

.sm-btn-accent:hover:not(:disabled) {
  filter: brightness(1.08);
}

.sm-btn-accent:disabled {
  opacity: 0.3;
  cursor: not-allowed;
}

.sm-btn-ico {
  font-size: 14px;
}

.sm-fade-enter-active,
.sm-fade-leave-active {
  transition: opacity 0.18s ease;
}
.sm-fade-enter-from,
.sm-fade-leave-to {
  opacity: 0;
}
</style>
