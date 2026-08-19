<template>
  <transition name="modal-fade">
    <div v-if="visible" class="close-overlay" @mousedown.self.prevent="overlayPress()" @mouseup.self="overlayRelease() && $emit('close')" @mouseleave="overlayCancel()">
      <div class="close-modal" @click.stop>
        <button class="close-x" @click="$emit('close')" :aria-label="t('Cancel')">
          <i class="material-icons">close</i>
        </button>

        <div class="close-hero">
          <div class="close-icon">
            <i class="material-icons">logout</i>
          </div>
          <h2>{{ t('Close MoonHunt?') }}</h2>
          <p>{{ t('MoonHunt can keep running in the system tray so your downloads continue in the background.') }}</p>
        </div>

        <div class="close-options">
          <button type="button" class="close-option tray" @click="stayInTray">
            <span class="close-option-icon">
              <i class="material-icons">minimize</i>
            </span>
            <span class="close-option-copy">
              <strong>{{ t('Stay in system tray') }}</strong>
              <small>{{ t('Hide to the Windows tray and keep downloading.') }}</small>
            </span>
            <i class="material-icons close-option-arrow">chevron_right</i>
          </button>

          <button type="button" class="close-option quit" @click="quit">
            <span class="close-option-icon">
              <i class="material-icons">power_settings_new</i>
            </span>
            <span class="close-option-copy">
              <strong>{{ t('Exit completely') }}</strong>
              <small>{{ t('Close the app and stop all downloads.') }}</small>
            </span>
            <i class="material-icons close-option-arrow">chevron_right</i>
          </button>
        </div>

        <div class="close-footer">
          <button type="button" class="close-cancel" @click="$emit('close')">{{ t('Cancel') }}</button>
        </div>
      </div>
    </div>
  </transition>
</template>

<script setup lang="ts">
import { onBeforeUnmount, onMounted } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { useI18n } from '../i18n'
import { useOverlayClose } from '../composables/useOverlayClose'

const props = defineProps<{ visible: boolean }>()
const emit = defineEmits<{ close: [] }>()

const { t } = useI18n()
const { overlayPress, overlayCancel, overlayRelease } = useOverlayClose()

async function stayInTray() {
  try {
    await invoke('hide_to_tray')
    emit('close')
  } catch (error) {
    console.error('Failed to hide MoonHunt to the system tray:', error)
  }
}

function quit() {
  invoke('quit_app').catch((error) => {
    console.error('Failed to quit MoonHunt:', error)
  })
}

function handleKeydown(e: KeyboardEvent) {
  if (e.key === 'Escape' && props.visible) {
    emit('close')
  }
}

onMounted(() => document.addEventListener('keydown', handleKeydown))
onBeforeUnmount(() => document.removeEventListener('keydown', handleKeydown))
</script>

<style scoped>
.close-overlay {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.55);
  backdrop-filter: blur(5px);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 2400;
  padding: 20px;
}

.close-modal {
  position: relative;
  width: min(440px, 92vw);
  border-radius: 18px;
  background: var(--bg-card);
  border: 1px solid var(--border-strong);
  box-shadow: var(--shadow-lg);
  overflow: hidden;
}

.close-x {
  position: absolute;
  top: 12px;
  right: 12px;
  width: 30px;
  height: 30px;
  background: var(--hover-bg);
  border: none;
  border-radius: 8px;
  color: var(--text-dim);
  display: flex;
  align-items: center;
  justify-content: center;
  cursor: pointer;
  transition: background 0.15s, color 0.15s;
  z-index: 1;
}

.close-x:hover {
  background: rgba(239, 83, 80, 0.12);
  color: var(--status-error);
}

.close-x .material-icons {
  font-size: 16px;
}

.close-hero {
  padding: 30px 26px 18px;
  text-align: center;
}

.close-icon {
  width: 52px;
  height: 52px;
  margin: 0 auto 10px;
  border-radius: 50%;
  background: rgba(var(--accent-rgb), 0.09);
  border: 1px solid rgba(var(--accent-rgb), 0.18);
  display: flex;
  align-items: center;
  justify-content: center;
}

.close-icon .material-icons {
  font-size: 24px;
  color: var(--accent-strong);
}

.close-hero h2 {
  margin: 0;
  font-size: 19px;
  font-weight: 750;
  letter-spacing: -0.02em;
}

.close-hero p {
  margin: 7px 0 0;
  color: var(--text-muted);
  font-size: 12.5px;
  line-height: 1.55;
  max-width: 340px;
  margin-left: auto;
  margin-right: auto;
}

.close-options {
  display: flex;
  flex-direction: column;
  gap: 8px;
  padding: 4px 18px 0;
}

.close-option {
  width: 100%;
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 12px 13px;
  border-radius: 13px;
  background: var(--hover-bg);
  border: 1px solid var(--border);
  color: inherit;
  font: inherit;
  text-align: start;
  cursor: pointer;
  transition: border-color 0.15s, background 0.15s, transform 0.12s;
}

.close-option:hover {
  transform: translateY(-1px);
}

.close-option:active {
  transform: scale(0.985);
}

.close-option.tray:hover {
  background: rgba(var(--accent-rgb), 0.07);
  border-color: rgba(var(--accent-rgb), 0.3);
}

.close-option.quit:hover {
  background: rgba(239, 83, 80, 0.07);
  border-color: rgba(239, 83, 80, 0.3);
}

.close-option-icon {
  width: 38px;
  height: 38px;
  border-radius: 10px;
  background: rgba(var(--accent-rgb), 0.08);
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
}

.close-option.tray .close-option-icon {
  color: var(--accent-strong);
}

.close-option.quit .close-option-icon {
  background: rgba(239, 83, 80, 0.09);
  color: var(--status-error);
}

.close-option-icon .material-icons {
  font-size: 20px;
}

.close-option-copy {
  min-width: 0;
  flex: 1;
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.close-option-copy strong {
  font-size: 13px;
  font-weight: 650;
}

.close-option-copy small {
  color: var(--text-dim);
  font-size: 11px;
  line-height: 1.4;
}

.close-option-arrow {
  color: var(--text-dim);
  font-size: 18px;
  flex-shrink: 0;
}

.close-footer {
  padding: 12px 18px 18px;
  display: flex;
  justify-content: center;
}

.close-cancel {
  background: none;
  border: none;
  color: var(--text-dim);
  font-size: 12px;
  font-weight: 600;
  cursor: pointer;
  padding: 8px 16px;
  border-radius: 8px;
  transition: background 0.15s, color 0.15s;
}

.close-cancel:hover {
  background: var(--hover-bg);
  color: var(--text);
}

@media (max-width: 520px) {
  .close-overlay {
    padding: 0;
    align-items: flex-end;
  }

  .close-modal {
    width: 100%;
    max-width: none;
    border-radius: 20px 20px 0 0;
  }
}
</style>
