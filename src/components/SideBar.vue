<template>
  <div class="sidebar">
    <div class="sidebar-logo">
      <i class="material-icons" style="font-size: 32px; color: var(--accent-strong);">nightlight_round</i>
    </div>

    <div class="sidebar-nav">
      <div
        v-for="item in menuItems"
        :key="item.key"
        @click="item.action === 'about' ? $emit('about') : $emit('navigate', item.path)"
        class="menu-item"
        :class="{ active: item.action ? false : activeRoute === item.path }"
      >
        <i class="material-icons" style="font-size: 24px;">{{ item.icon }}</i>
        <span class="menu-label">{{ item.label }}</span>
      </div>
    </div>

    <button
      class="menu-item speed-limit-toggle"
      :class="{ active: globalSpeedLimitEnabled, disabled: isApplying }"
      :title="globalSpeedLimitEnabled ? t('Speed limit is on') : t('Speed limit is off')"
      @click="onSpeedLimitToggle"
      @contextmenu.prevent="openSpeedPopover"
    >
      <i class="material-icons" style="font-size: 24px;">speed</i>
      <span class="speed-limit-value">{{ speedLimitText }}</span>
    </button>

    <template v-if="speedLimitPopover">
      <div class="sl-overlay" @click="closeSpeedPopover" @contextmenu.prevent="closeSpeedPopover"></div>
      <div class="sl-popover" :style="popoverStyle">
        <div class="sl-pop-header">
          <span class="sl-pop-title">{{ t('Speed Limit') }}</span>
          <div class="speed-toggle" :class="{ on: globalSpeedLimitEnabled }" @click="toggleSpeedLimitFromPopover">
            <div class="speed-toggle-knob"></div>
          </div>
        </div>
        <transition name="speed-slide">
          <div v-if="globalSpeedLimitEnabled" class="speed-control-row">
            <div class="stepper speed-stepper">
              <button @click="stepSpeedLimit(-1)"><i class="material-icons">remove</i></button>
              <input v-model.number="speedInputValue" type="number" min="0.01" step="0.01" :style="{ width: Math.max(40, 18 + String(speedInputValue).length * 9) + 'px' }" @change="applySpeedFromPopover" />
              <button @click="stepSpeedLimit(1)"><i class="material-icons">add</i></button>
            </div>
            <div class="speed-unit-switch">
              <button v-for="u in (['KB','MB','GB'] as const)" :key="u" type="button" class="speed-unit-btn" :class="{ active: speedUnit === u }" @click="changeUnit(u)">{{ u }}</button>
            </div>
          </div>
        </transition>
      </div>
    </template>

    <div class="sidebar-version" dir="ltr">v{{ appVersion }}</div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { useI18n } from '../i18n'
import { useGlobalSpeedLimit } from '../composables/useGlobalSpeedLimit'

defineProps<{ activeRoute: string }>()
defineEmits<{ navigate: [path: string]; about: [] }>()

const { t } = useI18n()
const appVersion = ref('')
const { globalSpeedLimitEnabled, globalSpeedLimitKb, isApplying, loadGlobalSpeedLimit, setGlobalSpeedLimit, toggleGlobalSpeedLimit } = useGlobalSpeedLimit()

const speedLimitText = computed(() => {
  if (!globalSpeedLimitEnabled.value || globalSpeedLimitKb.value <= 0) return t('Unlimited')
  if (globalSpeedLimitKb.value >= 1_000_000) {
    const gb = globalSpeedLimitKb.value / 1_000_000
    return `${gb === Math.floor(gb) ? gb : gb.toFixed(2)} GB/s`
  }
  if (globalSpeedLimitKb.value >= 1000) {
    const mb = globalSpeedLimitKb.value / 1000
    return `${mb === Math.floor(mb) ? mb : mb.toFixed(2)} MB/s`
  }
  return `${globalSpeedLimitKb.value} KB/s`
})

async function onSpeedLimitToggle() {
  try {
    await toggleGlobalSpeedLimit()
  } catch (error) {
    console.error('Failed to toggle global speed limit:', error)
  }
}

// -- Speed limit popover (right-click) --
const speedLimitPopover = ref(false)
const popoverPos = ref({ x: 0, y: 0 })
const speedInputValue = ref(200)
const speedUnit = ref<'KB' | 'MB' | 'GB'>('KB')
const savedSpeedKb = ref(0)

const multipliers = { KB: 1, MB: 1024, GB: 1024 * 1024 }
const stepSize = computed(() => speedUnit.value === 'KB' ? 100 : 1)

const popoverStyle = computed(() => ({
  left: popoverPos.value.x + 'px',
  bottom: (window.innerHeight - popoverPos.value.y + 8) + 'px',
}))

function openSpeedPopover(e: MouseEvent) {
  savedSpeedKb.value = globalSpeedLimitKb.value
  const kb = globalSpeedLimitKb.value || 0
  if (kb >= 1024 * 1024) { speedUnit.value = 'GB'; speedInputValue.value = Math.round(kb / (1024 * 1024)) }
  else if (kb >= 1024) { speedUnit.value = 'MB'; speedInputValue.value = Math.round(kb / 1024) }
  else { speedUnit.value = 'KB'; speedInputValue.value = kb || 200 }
  popoverPos.value = { x: e.clientX, y: e.clientY }
  speedLimitPopover.value = true
}

function closeSpeedPopover() {
  if (!speedInputValue.value || speedInputValue.value <= 0) {
    speedInputValue.value = savedSpeedKb.value || 200
  }
  speedLimitPopover.value = false
}

function stepSpeedLimit(dir: number) {
  speedInputValue.value = Math.max(0.01, speedInputValue.value + dir * stepSize.value)
  applySpeedFromPopover()
}

function changeUnit(u: 'KB' | 'MB' | 'GB') {
  const currentKb = speedInputValue.value * multipliers[speedUnit.value]
  speedUnit.value = u
  speedInputValue.value = Math.max(0.01, parseFloat((currentKb / multipliers[u]).toFixed(2)))
}

async function applySpeedFromPopover() {
  const val = speedInputValue.value
  if (!val || val <= 0) return
  const kb = val * multipliers[speedUnit.value]
  await setGlobalSpeedLimit(globalSpeedLimitEnabled.value, kb)
}

async function toggleSpeedLimitFromPopover() {
  await toggleGlobalSpeedLimit()
  if (globalSpeedLimitEnabled.value) await applySpeedFromPopover()
}

onMounted(async () => {
  void loadGlobalSpeedLimit()
  invoke<string>('get_app_version').then(v => {
    appVersion.value = v
  }).catch(() => {})
})

const menuItems = computed(() => [
  { key: 'home', label: t('Home'), icon: 'home', path: '/' },
  { key: 'downloads', label: t('Downloads'), icon: 'cloud_download', path: '/downloads' },
  { key: 'media', label: t('Media'), icon: 'movie', path: '/media' },
  { key: 'addon', label: t('Addon'), icon: 'extension', path: '/addon' },
  { key: 'settings', label: t('Settings'), icon: 'settings', path: '/settings' },
  { key: 'about', label: t('About'), icon: 'info', action: 'about' as const },
])
</script>

<style scoped>
.sidebar {
  width: var(--sidebar-width);
  background: var(--bg-card);
  display: flex;
  flex-direction: column;
  align-items: center;
  padding: 16px 0;
  flex-shrink: 0;
  border-right: 1px solid var(--border);
  height: 100vh;
  overflow: hidden;
}

.sidebar-logo {
  margin-bottom: 32px;
}

.sidebar-nav {
  display: flex;
  flex-direction: column;
  gap: 0;
  width: 100%;
  flex: 1;
}

.menu-item {
  display: flex;
  flex-direction: column;
  align-items: center;
  padding: 12px 0;
  cursor: pointer;
  transition: all 0.2s;
  width: 100%;
  border-radius: 0;
  color: var(--text-dim);
}

.menu-item:hover {
  background: rgba(var(--accent-rgb), 0.08);
  color: var(--accent-strong);
}

.menu-item.active {
  background: var(--bg-card);
  color: var(--accent-strong);
}

.speed-limit-toggle {
  background: transparent;
  border: none;
  font-family: inherit;
}

.speed-limit-toggle.disabled {
  opacity: 0.6;
  pointer-events: none;
}

.speed-limit-toggle:hover {
  background: transparent;
  color: var(--text);
  opacity: 0.85;
  transform: scale(1.06);
}

.speed-limit-toggle.active:hover {
  color: var(--accent-strong);
  opacity: 1;
  transform: scale(1.06);
}

.speed-limit-value {
  display: block;
  max-width: 62px;
  margin-top: 3px;
  font-size: 9px;
  line-height: 1.1;
  font-weight: 600;
  color: var(--text-dim);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.speed-limit-toggle.active .speed-limit-value {
  color: var(--accent-strong);
}

.menu-label {
  font-size: 11px;
  margin-top: 4px;
  font-weight: 500;
}

.sidebar-version {
  color: var(--text-dim);
  font-size: 10px;
  padding-top: 8px;
}

.sl-overlay { position: fixed; inset: 0; z-index: 999; }
.sl-popover { position: fixed; z-index: 1000; background: var(--bg-card); border: 1px solid var(--border); border-radius: 12px; padding: 14px 16px; box-shadow: 0 8px 24px rgba(0,0,0,0.25); width: fit-content; }
.sl-pop-header { display: flex; align-items: center; justify-content: space-between; gap: 12px; margin-bottom: 10px; }
.sl-pop-title { font-size: 13px; font-weight: 600; color: var(--text); }
.speed-toggle { width: 42px; height: 24px; border-radius: 12px; background: var(--hover-bg); border: 1px solid var(--border-strong); cursor: pointer; position: relative; transition: background 0.25s ease, border-color 0.25s ease; flex-shrink: 0; }
.speed-toggle.on { background: rgba(var(--accent-rgb), 0.25); border-color: rgba(var(--accent-rgb), 0.4); }
.speed-toggle-knob { position: absolute; top: 2px; left: 2px; width: 18px; height: 18px; border-radius: 50%; background: var(--text-dim); transition: transform 0.25s cubic-bezier(0.34, 1.4, 0.64, 1), background 0.25s ease; }
.speed-toggle.on .speed-toggle-knob { transform: translateX(18px); background: var(--accent); }
.speed-control-row { display: flex; align-items: stretch; gap: 8px; }
.stepper { display: flex; width: fit-content; border-radius: 8px; overflow: hidden; border: 1px solid var(--border-strong); }
.stepper button { width: 36px; height: 38px; border: none; background: var(--hover-bg); color: var(--text-dim); cursor: pointer; display: flex; align-items: center; justify-content: center; transition: background 0.15s ease, color 0.15s ease; }
.stepper button:hover { background: rgba(var(--accent-rgb), 0.1); color: var(--accent-strong); }
.stepper input { text-align: center; border: none; border-left: 1px solid var(--border-strong); border-right: 1px solid var(--border-strong); background: transparent; color: var(--text); font-size: 13px; font-family: inherit; outline: none; -moz-appearance: textfield; }
.stepper input::-webkit-outer-spin-button,
.stepper input::-webkit-inner-spin-button { -webkit-appearance: none; margin: 0; }
.stepper input:focus { box-shadow: none; }
.speed-unit-switch { display: flex; height: 38px; border: 1px solid var(--border-strong); border-radius: 8px; overflow: hidden; }
.speed-unit-btn { min-width: 44px; border: none; border-right: 1px solid var(--border-strong); background: var(--hover-bg); color: var(--text-dim); font-size: 12px; font-weight: 600; cursor: pointer; transition: background 0.15s ease, color 0.15s ease; font-family: inherit; }
.speed-unit-btn:last-child { border-right: none; }
.speed-unit-btn:hover { background: rgba(var(--accent-rgb), 0.1); color: var(--accent-strong); }
.speed-unit-btn.active { background: rgba(var(--accent-rgb), 0.12); color: var(--accent-strong); }
.speed-slide-enter-active { transition: opacity 0.2s ease, transform 0.2s ease; }
.speed-slide-leave-active { transition: opacity 0.15s ease, transform 0.15s ease; }
.speed-slide-enter-from { opacity: 0; transform: translateY(-6px); }
.speed-slide-leave-to { opacity: 0; transform: translateY(-4px); }
</style>
