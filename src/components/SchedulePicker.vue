<template>
  <div class="sp-schedule" :class="{ enabled }">
    <div class="sp-row">
      <div class="sp-left">
        <i class="material-icons">schedule</i>
        <span>{{ t('Schedule') }}</span>
      </div>
      <div class="sp-right">
        <button v-if="enabled" class="sp-configure" @click="openSchedule">
          <i class="material-icons">event</i>
          {{ t('Configure') }}
        </button>
        <div
          class="sp-toggle"
          :class="{ active: enabled }"
          @click="toggle"
          role="switch"
          :aria-checked="enabled"
        >
          <span class="sp-toggle-track"><span class="sp-toggle-thumb"></span></span>
        </div>
      </div>
    </div>

    <div v-if="enabled" class="sp-controls">
      <div v-if="schedule" class="sp-preview">
        <i class="material-icons">event_available</i>
        <span>{{ schedulePreview }}</span>
      </div>
      <div v-else class="sp-hint">{{ t('Configure a date and time...') }}</div>
    </div>

    <ScheduleModal ref="scheduleModalRef" />
  </div>
</template>

<script setup lang="ts">
import { ref, computed } from 'vue'
import ScheduleModal, { type ScheduleResult } from './ScheduleModal.vue'
import { useI18n } from '../i18n'

const props = defineProps<{
  enabled: boolean
  schedule: ScheduleResult | null
}>()

const emit = defineEmits<{
  'update:enabled': [value: boolean]
  'update:schedule': [value: ScheduleResult | null]
}>()
const { t } = useI18n()

const scheduleModalRef = ref<InstanceType<typeof ScheduleModal> | null>(null)

const schedulePreview = computed(() => {
  if (!props.schedule) return ''
  const sched = props.schedule
  const date = new Date(sched.scheduledTime)
  const dateStr = date.toLocaleDateString(undefined, { month: 'short', day: 'numeric', year: 'numeric' })
  const timeStr = date.toLocaleTimeString(undefined, { hour: '2-digit', minute: '2-digit' })
  const action = sched.action === 'pause' ? t('Pause') : sched.action === 'resume' ? t('Resume') : t('Start')
  const repeat = repeatLabel(sched.repeatInterval)
  return `${action} - ${dateStr} at ${timeStr} - ${repeat}`
})

function repeatLabel(ms: number): string {
  if (ms === 60 * 60 * 1000) return t('Hourly')
  if (ms === 24 * 60 * 60 * 1000) return t('Daily')
  if (ms === 7 * 24 * 60 * 60 * 1000) return t('Weekly')
  return t('Once')
}

function toggle() {
  const next = !props.enabled
  emit('update:enabled', next)
  if (!next) emit('update:schedule', null)
}

async function openSchedule() {
  const result = await scheduleModalRef.value?.open()
  if (result) emit('update:schedule', result)
}
</script>

<style scoped>
.sp-schedule {
  background: rgba(255, 255, 255, 0.02);
  border: 1px solid var(--border);
  border-radius: 12px;
  padding: 12px 14px;
  transition: all 0.25s ease;
}

.sp-schedule.enabled {
  background: rgba(var(--accent-rgb), 0.04);
  border-color: rgba(var(--accent-rgb), 0.18);
}

.sp-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 10px;
}

.sp-left {
  display: flex;
  align-items: center;
  gap: 8px;
  color: var(--text-muted);
  font-size: 13px;
}

.sp-left .material-icons {
  font-size: 18px;
  color: var(--text-dim);
}

.sp-schedule.enabled .sp-left {
  color: var(--text);
}

.sp-schedule.enabled .sp-left .material-icons {
  color: var(--accent-strong);
}

.sp-right {
  display: flex;
  align-items: center;
  gap: 8px;
}

.sp-configure {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  background: none;
  border: none;
  color: var(--accent-strong);
  font-size: 11.5px;
  font-weight: 700;
  font-family: inherit;
  cursor: pointer;
  padding: 4px 8px;
  border-radius: 6px;
  transition: background 0.15s;
}

.sp-configure:hover {
  background: rgba(var(--accent-rgb), 0.1);
}

.sp-configure i {
  font-size: 14px;
}

.sp-toggle {
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

.sp-toggle.active {
  background: rgba(var(--accent-rgb), 0.25);
  border-color: rgba(var(--accent-rgb), 0.4);
}

.sp-toggle-track {
  position: absolute;
  top: 2px;
  left: 2px;
  width: 18px;
  height: 18px;
  border-radius: 50%;
  transition: transform 0.25s cubic-bezier(0.34, 1.4, 0.64, 1);
}

.sp-toggle.active .sp-toggle-track {
  transform: translateX(18px);
}

.sp-toggle-thumb {
  position: absolute;
  inset: 0;
  border-radius: 50%;
  background: var(--text-dim);
  transition: background 0.25s ease;
}

.sp-toggle.active .sp-toggle-thumb {
  background: var(--accent);
}

.sp-controls {
  margin-top: 11px;
  padding-top: 11px;
  border-top: 1px solid var(--border);
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.sp-preview {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 12px;
  color: var(--text-muted);
}

.sp-preview i {
  font-size: 15px;
  color: var(--accent-strong);
}

.sp-hint {
  font-size: 11px;
  color: var(--text-dim);
}
</style>
