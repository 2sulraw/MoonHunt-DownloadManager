<template>
  <div class="addon-layout" :class="{ collapsed: isCollapsed }">
    <aside class="addon-sidebar">
      <div class="sidebar-header">
        <h2 class="sidebar-title">{{ t('Addon') }}</h2>
        <button
          class="header-icon-btn"
          @click="openAddonsFolder"
          :title="t('Open Addons Folder')"
          :aria-label="t('Open Addons Folder')"
        >
          <i class="material-icons">folder_open</i>
        </button>
      </div>
      <button
        class="collapse-btn"
        @click="isCollapsed = !isCollapsed"
        :title="isCollapsed ? t('Expand') : t('Collapse')"
        :aria-label="isCollapsed ? t('Expand') : t('Collapse')"
      >
        <i class="material-icons">{{ isCollapsed ? 'chevron_right' : 'chevron_left' }}</i>
      </button>
      <nav class="sidebar-nav">
        <button
          class="sidebar-item"
          :class="{ active: activeTab === 'general' }"
          @click="activeTab = 'general'"
        >
          <i class="material-icons">tune</i>
          <span>{{ t('General') }}</span>
        </button>
        <button
          v-for="tab in tabs"
          :key="tab.id"
          class="sidebar-item"
          :class="{ active: activeTab === tab.id }"
          @click="activeTab = tab.id"
          :title="tab.name"
        >
          <i class="material-icons">{{ tab.icon }}</i>
          <span>{{ tab.name }}</span>
        </button>
      </nav>
      <div class="sidebar-footer">
        <button class="sidebar-add-btn" @click="showPicker = true">
          <i class="material-icons" style="font-size:14px">add</i>
          {{ t('Add') }}
        </button>
        <button class="sidebar-remove-btn" @click="onRemove">
          <i class="material-icons" style="font-size:14px">remove</i>
          {{ t('Remove') }}
        </button>
      </div>
    </aside>

    <main class="addon-content">
      <section v-show="activeTab === 'general'" class="addon-section">
        <h2 class="section-title">{{ t('General') }}</h2>
        <p class="section-sub">{{ t('Installed addons. Use Add to install more.') }}</p>
        <ul class="addon-manage-list">
          <li v-for="a in allKnown" :key="a.id" class="addon-manage-row">
            <div class="addon-row-icon">
              <i class="material-icons">{{ a.icon }}</i>
            </div>
            <div class="addon-row-main">
              <div class="addon-row-name">
                {{ a.name }}
                <small class="addon-version">v{{ a.manifest.version }}</small>
              </div>
              <div class="addon-row-desc">{{ a.manifest.description || a.manifest.author }}</div>
            </div>
            <button
              class="speed-toggle"
              :class="{ on: a.enabled }"
              @click="toggleEnabled(a.id, !a.enabled)"
              :title="a.enabled ? t('Disable') : t('Enable')"
              :aria-label="a.enabled ? t('Disable') : t('Enable')"
            >
              <div class="speed-toggle-knob"></div>
            </button>
            <button
              class="row-icon-btn"
              @click="shareAddon(a.id)"
              :title="t('Export as .zip')"
              :aria-label="t('Export as .zip')"
            >
              <i class="material-icons">ios_share</i>
            </button>
          </li>
        </ul>
        <p v-if="!allKnown.length" class="section-sub">{{ t('No addons yet.') }}</p>
      </section>

      <template v-for="tab in tabs" :key="tab.id">
        <div v-show="activeTab === tab.id" class="addon-tab-body">
          <AddonHost :key="`${tab.id}:${hostKey}`" :addon-id="tab.id" :initial-url="pendingUrlFor(tab.id)" />
        </div>
      </template>
    </main>

    <AddonPickerModal
      :visible="showPicker"
      @close="showPicker = false"
      @installed="onInstalled"
    />
  </div>
</template>

<script setup lang="ts">
import { inject, onActivated, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { useRoute } from 'vue-router'
import { open } from '@tauri-apps/plugin-dialog'
import { invoke } from '@tauri-apps/api/core'
import { useI18n } from '../i18n'
import { useAddons } from '../composables/useAddons'
import AddonPickerModal from '../components/AddonPickerModal.vue'
import AddonHost from '../components/AddonHost.vue'

const { t } = useI18n()
const notify = inject<(msg: string, type: 'success' | 'error' | 'info') => void>('notify', (m) => console.log(m))
const route = useRoute()
const { tabs, allKnown, refresh, uninstall, setEnabled, exportPackage } = useAddons()

const activeTab = ref('general')
const isCollapsed = ref(false)
const showPicker = ref(false)
const hostKey = ref(0)
const pendingUrls = ref<Record<string, string>>({})

function pendingUrlFor(id: string): string {
  return pendingUrls.value[id] || ''
}

function openAddon(id: string, url?: string) {
  if (url) pendingUrls.value[id] = url
  activeTab.value = id
}

function applyRouteQuery() {
  const q = route.query as Record<string, string | undefined>
  if (typeof q.addon === 'string' && q.addon) {
    openAddon(q.addon, typeof q.url === 'string' ? q.url : undefined)
  } else if (typeof q.url === 'string' && q.url) {
    const handled = tabs.value[0]
    if (handled) openAddon(handled.id, q.url)
  }
}

function onOpenAddonEvent(e: Event) {
  const detail = (e as CustomEvent).detail as { addonId?: string; url?: string }
  if (detail?.addonId) openAddon(detail.addonId, detail.url)
}

watch(() => route.query, () => applyRouteQuery())

onMounted(async () => {
  await refresh()
  applyRouteQuery()
  window.addEventListener('moonhunt:open-addon', onOpenAddonEvent)
})

onActivated(async () => {
  await refresh()
  if (activeTab.value !== 'general' && !tabs.value.some((x) => x.id === activeTab.value)) {
    activeTab.value = 'general'
  }
})

onBeforeUnmount(() => {
  window.removeEventListener('moonhunt:open-addon', onOpenAddonEvent)
})

function onInstalled(id: string) {
  showPicker.value = false
  hostKey.value += 1
  activeTab.value = id
}

async function onRemove() {
  if (activeTab.value === 'general') {
    notify(t('Select an addon tab first.'), 'info')
    return
  }
  const id = activeTab.value
  try {
    if (!window.confirm(t('Remove this addon and its files?'))) return
    const res = await uninstall(id)
    activeTab.value = 'general'
    notify(res, 'success')
  } catch (e) {
    notify(`${t('Failed to remove addon')}: ${String(e)}`, 'error')
  }
}

async function toggleEnabled(id: string, enabled: boolean) {
  try {
    await setEnabled(id, enabled)
    if (!enabled && activeTab.value === id) activeTab.value = 'general'
  } catch (e) {
    notify(`${t('Failed to update addon')}: ${String(e)}`, 'error')
  }
}

async function openAddonsFolder() {
  try {
    await invoke<string>('open_addons_folder')
  } catch (e) {
    notify(`${t('Failed to open addons folder')}: ${String(e)}`, 'error')
  }
}

async function shareAddon(id: string) {
  try {
    const dir = await open({
      multiple: false,
      directory: true,
      title: t('Choose folder for the exported .zip'),
    })
    if (!dir) return
    const out = await exportPackage(id, dir as string)
    notify(`${t('Addon exported')}: ${out}`, 'success')
  } catch (e) {
    notify(`${t('Failed to export addon')}: ${String(e)}`, 'error')
  }
}
</script>

<style scoped>
.addon-layout {
  display: flex;
  min-height: 100%;
  color: var(--text);
}

.addon-sidebar {
  position: fixed;
  top: 50px;
  left: var(--sidebar-width, 70px);
  width: 180px;
  height: calc(100vh - 50px);
  background: var(--bg-card);
  border-right: 1px solid var(--border);
  display: flex;
  flex-direction: column;
  overflow: visible;
  z-index: 10;
  transition: width 0.2s ease;
}

.addon-layout.collapsed .addon-sidebar {
  width: 0;
  border-right-color: transparent;
}

.addon-layout.collapsed .addon-content {
  margin-left: 0;
}

.addon-layout.collapsed .sidebar-header,
.addon-layout.collapsed .sidebar-nav,
.addon-layout.collapsed .sidebar-footer {
  opacity: 0;
  visibility: hidden;
  pointer-events: none;
}

.sidebar-header,
.sidebar-nav,
.sidebar-footer {
  transition: opacity 0.15s ease, visibility 0.15s ease;
}

.sidebar-header {
  display: flex;
  align-items: center;
  justify-content: flex-start;
  gap: 6px;
  padding: 20px 12px 16px 16px;
}

.header-icon-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 28px;
  height: 28px;
  flex-shrink: 0;
  border-radius: 8px;
  border: 1px solid transparent;
  background: transparent;
  color: var(--text-dim);
  cursor: pointer;
  padding: 0;
  transition: all 0.15s;
}

.header-icon-btn:hover {
  color: var(--accent-strong);
  border-color: var(--border);
  background: var(--hover-bg);
}

.header-icon-btn i {
  font-size: 18px;
}

.collapse-btn {
  position: absolute;
  top: 22px;
  right: -13px;
  display: flex;
  align-items: center;
  justify-content: center;
  width: 26px;
  height: 26px;
  flex-shrink: 0;
  border-radius: 50%;
  border: 1px solid var(--border);
  background: var(--bg-card);
  color: var(--text-dim);
  cursor: pointer;
  padding: 0;
  z-index: 11;
  box-shadow: var(--shadow-sm);
  transition: all 0.15s;
}

.collapse-btn:hover {
  color: var(--text);
  border-color: var(--border-strong);
  background: var(--hover-bg);
}

.collapse-btn i {
  font-size: 18px;
}

.sidebar-title {
  margin: 0;
  font-size: 16px;
  font-weight: 700;
  color: var(--text);
}

.sidebar-nav {
  flex: 1;
  overflow-y: auto;
  padding: 8px 12px;
}

.sidebar-item {
  display: flex;
  align-items: center;
  gap: 10px;
  width: 100%;
  padding: 10px 12px;
  border-radius: 8px;
  background: transparent;
  border: none;
  color: var(--text-dim);
  font-size: 14px;
  font-family: inherit;
  cursor: pointer;
  text-align: left;
  transition: all 0.15s;
  margin-bottom: 2px;
}

.sidebar-item:hover {
  background: var(--hover-bg);
  color: var(--text);
}

.sidebar-item.active {
  background: rgba(var(--accent-rgb), 0.1);
  color: var(--accent-strong);
  font-weight: 600;
}

.sidebar-item i {
  font-size: 20px;
  width: 22px;
  text-align: center;
  flex-shrink: 0;
}

.sidebar-item span {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.sidebar-footer {
  margin-top: auto;
  padding: 10px 12px 12px;
  border-top: 1px solid var(--border);
  display: flex;
  flex-direction: row;
  gap: 8px;
}

.sidebar-add-btn {
  flex: 1;
  min-width: 0;
  padding: 7px 4px;
  border-radius: 7px;
  background: var(--accent);
  color: #0a0e1a;
  font-weight: 700;
  font-size: 12px;
  border: none;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 4px;
  transition: opacity 0.15s;
}

.sidebar-add-btn:hover {
  opacity: 0.9;
}

.sidebar-remove-btn {
  flex: 1;
  min-width: 0;
  padding: 7px 4px;
  border-radius: 7px;
  background: var(--hover-bg);
  color: var(--text-dim);
  border: 1px solid var(--border);
  font-size: 12px;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 4px;
  transition: all 0.15s;
}

.sidebar-remove-btn:hover {
  color: var(--text);
  border-color: var(--border-strong);
}

.addon-content {
  flex: 1;
  overflow-y: auto;
  padding: 24px 32px 32px;
  margin-left: 180px;
  transition: margin-left 0.2s ease;
}

.addon-section {
  margin-bottom: 32px;
  padding-bottom: 24px;
  border-bottom: 1px solid var(--border);
  position: relative;
}

.addon-tab-body {
  display: flex;
  flex-direction: column;
  gap: 14px;
  width: min(720px, 100%);
  margin: 0 auto;
}

.section-title {
  margin: 0 0 4px;
  font-size: 18px;
  font-weight: 700;
  letter-spacing: -0.01em;
}

.section-sub {
  margin: 0 0 20px;
  color: var(--text-dim);
  font-size: 13px;
}

.addon-manage-list {
  list-style: none;
  margin: 0;
  padding: 0;
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.addon-manage-row {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 10px 12px;
  border: 1px solid var(--border);
  border-radius: 10px;
  background: var(--bg-card);
}

.addon-row-icon {
  width: 36px;
  height: 36px;
  border-radius: 10px;
  display: flex;
  align-items: center;
  justify-content: center;
  background: rgba(var(--accent-rgb), 0.12);
  color: var(--accent-strong);
  flex-shrink: 0;
}

.addon-row-main {
  flex: 1;
  min-width: 0;
}

.addon-row-name {
  font-weight: 700;
  font-size: 13px;
}

.addon-version {
  color: var(--text-dim);
  font-weight: 400;
  margin-left: 6px;
}

.addon-row-desc {
  font-size: 12px;
  color: var(--text-dim);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.speed-toggle {
  width: 42px;
  height: 24px;
  border-radius: 12px;
  background: var(--hover-bg);
  border: 1px solid var(--border-strong);
  cursor: pointer;
  position: relative;
  transition: background 0.25s ease, border-color 0.25s ease;
  flex-shrink: 0;
}
.speed-toggle.on {
  background: rgba(var(--accent-rgb), 0.25);
  border-color: rgba(var(--accent-rgb), 0.4);
}
.speed-toggle-knob {
  position: absolute;
  top: 2px;
  left: 2px;
  width: 18px;
  height: 18px;
  border-radius: 50%;
  background: var(--text-dim);
  transition: transform 0.25s cubic-bezier(0.34, 1.4, 0.64, 1), background 0.25s ease;
}
.speed-toggle.on .speed-toggle-knob {
  transform: translateX(18px);
  background: var(--accent);
}

.row-icon-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 30px;
  height: 30px;
  flex-shrink: 0;
  border-radius: 8px;
  border: 1px solid var(--border);
  background: transparent;
  color: var(--text-dim);
  cursor: pointer;
  padding: 0;
  transition: all 0.15s;
}

.row-icon-btn:hover {
  color: var(--accent-strong);
  border-color: rgba(var(--accent-rgb), 0.3);
  background: rgba(var(--accent-rgb), 0.08);
}

.row-icon-btn i {
  font-size: 17px;
}

@media (max-width: 700px) {
  .addon-layout {
    flex-direction: column;
  }
  .addon-sidebar {
    position: relative;
    top: 0;
    left: 0;
    width: 100%;
    height: auto;
    border-right: none;
    border-bottom: 1px solid var(--border);
    flex-direction: row;
    overflow-x: auto;
    z-index: auto;
  }
  .sidebar-header {
    display: none;
  }
  .collapse-btn {
    display: none;
  }
  .sidebar-nav {
    display: flex;
    gap: 4px;
    padding: 8px;
    overflow-y: visible;
  }
  .sidebar-item {
    white-space: nowrap;
    padding: 8px 12px;
  }
  .sidebar-footer {
    flex-direction: row;
    padding: 8px;
    border-top: none;
    border-left: 1px solid var(--border);
    margin-top: 0;
  }
  .addon-content {
    padding: 16px;
    margin-left: 0;
  }
  .addon-tab-body {
    width: 100%;
  }
}
</style>
