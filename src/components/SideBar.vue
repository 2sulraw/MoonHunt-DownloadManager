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
    >
      <i class="material-icons" style="font-size: 24px;">speed</i>
      <span class="speed-limit-value">{{ speedLimitText }}</span>
    </button>

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
const { globalSpeedLimitEnabled, globalSpeedLimitKb, isApplying, loadGlobalSpeedLimit, toggleGlobalSpeedLimit } = useGlobalSpeedLimit()

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
</style>
