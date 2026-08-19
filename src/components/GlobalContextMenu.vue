<!-- Global right-click context menu for downloads -->
<template>
  <Teleport to="body">
    <transition name="context-menu-fade">
      <div
        v-if="visible"
        class="global-context-menu"
        :style="clampedStyle"
        ref="menuRef"
        @click.stop
        @contextmenu.prevent
      >
        <div class="context-menu-header">
          <span class="context-menu-title">{{ download?.filename || t('Download') }}</span>
          <span class="context-menu-gid">{{ download?.gid?.slice(0, 8) || '' }}</span>
        </div>

        <hr class="context-menu-divider" />

        <button
          v-for="item in menuItems"
          :key="item.action"
          class="context-menu-item"
          :class="{ disabled: item.disabled }"
          @click="handleAction(item.action, $event)"
          @contextmenu.prevent
        >
          <i class="material-icons">{{ item.icon }}</i>
          <span>{{ item.label }}</span>
          <i v-if="item.shortcut" class="material-icons shortcut-icon">keyboard_arrow_right</i>
        </button>

        <hr v-if="hasDangerActions" class="context-menu-divider" />

        <button
          v-if="hasDangerActions"
          class="context-menu-item danger"
          @click="handleAction('remove')"
          @contextmenu.prevent
        >
          <i class="material-icons">delete</i>
          <span>{{ t('Remove') }}</span>
        </button>

        <button
          v-if="download?.status === 'error'"
          class="context-menu-item danger"
          @click="handleAction('clearError')"
          @contextmenu.prevent
        >
          <i class="material-icons">error_outline</i>
          <span>{{ t('Clear Error') }}</span>
        </button>
      </div>
    </transition>

    <!-- Group submenu -->
    <transition name="context-menu-fade">
      <div
        v-if="showGroupSubmenu"
        class="global-context-menu"
        :style="submenuStyle"
        ref="submenuRef"
        @click.stop
        @contextmenu.prevent
      >
        <div class="context-menu-header">
          <button class="submenu-back" @click="showGroupSubmenu = false">
            <i class="material-icons">arrow_back</i>
          </button>
          <span class="context-menu-title">{{ t('Add to Group') }}</span>
        </div>
        <hr class="context-menu-divider" />
        <button
          v-for="g in groups"
          :key="g.id"
          class="context-menu-item"
          @click="handleGroupSelect(g.id)"
          @contextmenu.prevent
        >
          <i class="material-icons" :style="{ color: g.color }">folder</i>
          <span>{{ g.name }}</span>
        </button>
      </div>
    </transition>
  </Teleport>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onBeforeUnmount, watch, nextTick } from 'vue'
import type { Download } from '../types/download'
import { useI18n } from '../i18n'

const { t } = useI18n()

interface ContextMenuItem {
  action: string
  label: string
  icon: string
  disabled?: boolean
  shortcut?: string
}

interface Props {
  download: Download | null
  visible: boolean
  position: { x: number; y: number }
  groups?: { id: string; name: string; color: string; memberGids: string[]; collapsed: boolean }[]
}

interface Emits {
  (e: 'close'): void
  (e: 'action', action: string, download: Download): void
  (e: 'add-to-group', groupId: string, download: Download): void
}

const props = withDefaults(defineProps<Props>(), { groups: () => [] })
const emit = defineEmits<Emits>()

const menuRef = ref<HTMLElement | null>(null)
const submenuRef = ref<HTMLElement | null>(null)
const showGroupSubmenu = ref(false)
const submenuAnchor = ref({ x: 0, y: 0 })
const menuHeight = ref(300)
const submenuHeight = ref(300)

async function remeasureMenus() {
  await nextTick()
  await new Promise<void>((resolve) => requestAnimationFrame(() => resolve()))
  if (menuRef.value) {
    const height = menuRef.value.offsetHeight
    if (height > 0) menuHeight.value = height
  }
  if (submenuRef.value) {
    const height = submenuRef.value.offsetHeight
    if (height > 0) submenuHeight.value = height
  }
}

const clampedStyle = computed(() => {
  const x = props.position.x
  const y = props.position.y
  const vw = typeof window !== 'undefined' ? window.innerWidth : 1920
  const vh = typeof window !== 'undefined' ? window.innerHeight : 1080
  const menuW = 220
  const clampedX = Math.min(Math.max(x, 8), vw - menuW - 8)
  const clampedY = Math.min(Math.max(y, 8), Math.max(8, vh - menuHeight.value - 8))
  return { top: clampedY + 'px', left: clampedX + 'px' }
})

const submenuStyle = computed(() => {
  const x = submenuAnchor.value.x + 4
  const y = submenuAnchor.value.y
  const vw = typeof window !== 'undefined' ? window.innerWidth : 1920
  const vh = typeof window !== 'undefined' ? window.innerHeight : 1080
  const menuW = 200
  const clampedX = Math.min(Math.max(x, 8), vw - menuW - 8)
  const clampedY = Math.min(Math.max(y, 8), Math.max(8, vh - submenuHeight.value - 8))
  return { top: clampedY + 'px', left: clampedX + 'px' }
})

const menuItems = computed<ContextMenuItem[]>(() => {
  if (!props.download) return []

  const items: ContextMenuItem[] = []

  if (props.download.status === 'active' || props.download.status === 'waiting') {
    items.push({ action: 'pause', label: t('Pause'), icon: 'pause' })
    items.push({ action: 'cancel', label: t('Cancel'), icon: 'cancel' })
  } else if (props.download.status === 'paused') {
    items.push({ action: 'resume', label: t('Resume'), icon: 'play_arrow' })
    items.push({ action: 'cancel', label: t('Cancel'), icon: 'cancel' })
  }

  if (props.download.status === 'error' || props.download.status === 'cancelled') {
    items.push({ action: 'retry', label: t('Retry'), icon: 'refresh' })
  }

  if (props.download.status === 'complete') {
    items.push({ action: 'redownload', label: t('Redownload'), icon: 'download' })
    items.push({ action: 'verifyHash', label: t('Verify Hash'), icon: 'verified_user' })
  }

  if (props.download.status !== 'complete' && props.groups && props.groups.length > 0) {
    items.push({ action: 'addToGroup', label: t('Add to Group'), icon: 'playlist_add', shortcut: 'group' })
  }

  items.push(
    { action: 'showInFolder', label: t('Show in Folder'), icon: 'folder_open' },
    { action: 'copyUrl', label: t('Copy URL'), icon: 'link' },
    { action: 'copyFilename', label: t('Copy Filename'), icon: 'content_copy' },
    { action: 'details', label: t('Details'), icon: 'info', shortcut: 'Enter' }
  )

  return items
})

const hasDangerActions = computed(() => {
  if (!props.download) return false
  return ['complete', 'error', 'paused', 'cancelled'].includes(props.download.status)
})

function handleAction(action: string, event?: MouseEvent) {
  if (!props.download) return
  if (action === 'addToGroup') {
    if (event) {
      const btn = (event.target as HTMLElement).closest('.context-menu-item') as HTMLElement
      if (btn) {
        const rect = btn.getBoundingClientRect()
        submenuAnchor.value = { x: rect.right, y: rect.top }
      }
    }
    showGroupSubmenu.value = true
    void remeasureMenus()
    return
  }
  emit('action', action, props.download)
  close()
}

function handleGroupSelect(groupId: string) {
  if (!props.download) return
  emit('add-to-group', groupId, props.download)
  showGroupSubmenu.value = false
  close()
}

function close() {
  showGroupSubmenu.value = false
  emit('close')
}

function handleClickOutside(event: MouseEvent) {
  if (event.button === 2) return
  if (menuRef.value && !menuRef.value.contains(event.target as Node)) {
    close()
  }
}

function handleKeydown(event: KeyboardEvent) {
  if (event.key === 'Escape') {
    close()
  }
  if (event.key === 'Enter' && props.download) {
    emit('action', 'details', props.download)
    close()
  }
}

onMounted(() => {
  document.addEventListener('mousedown', handleClickOutside)
  document.addEventListener('keydown', handleKeydown)
})

onBeforeUnmount(() => {
  document.removeEventListener('mousedown', handleClickOutside)
  document.removeEventListener('keydown', handleKeydown)
})

watch(() => props.visible, () => {
  showGroupSubmenu.value = false
  if (props.visible) void remeasureMenus()
})

watch(() => props.position, () => {
  showGroupSubmenu.value = false
  void remeasureMenus()
})

watch(() => props.groups?.length, () => {
  if (showGroupSubmenu.value) void remeasureMenus()
})
</script>

<style scoped>
.context-menu-fade-enter-active,
.context-menu-fade-leave-active {
  transition: opacity 0.12s ease, transform 0.12s ease;
}

.context-menu-fade-enter-from,
.context-menu-fade-leave-to {
  opacity: 0;
  transform: scale(0.95);
}

.global-context-menu {
  position: fixed;
  min-width: 200px;
  max-width: 280px;
  max-height: calc(100vh - 16px);
  overflow-y: auto;
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: 10px;
  box-shadow: var(--shadow-lg);
  font-size: 13px;
  color: var(--text);
  pointer-events: auto;
  z-index: 10000;
}

.context-menu-header {
  padding: 10px 12px 8px;
  border-bottom: 1px solid var(--border);
  display: flex;
  align-items: center;
  gap: 8px;
}

.submenu-back {
  background: none;
  border: none;
  color: var(--text-dim);
  cursor: pointer;
  padding: 0;
  display: flex;
  align-items: center;
  border-radius: 4px;
  transition: color 0.15s;
}
.submenu-back:hover { color: var(--text); }
.submenu-back .material-icons { font-size: 18px; }

.context-menu-title {
  display: block;
  font-weight: 600;
  font-size: 12px;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  color: var(--text);
}

.context-menu-gid {
  display: block;
  font-size: 10px;
  color: var(--text-dim);
  font-family: monospace;
  margin-top: 2px;
}

.context-menu-divider {
  margin: 0;
  border: none;
  border-top: 1px solid var(--border);
}

.context-menu-item {
  display: flex;
  align-items: center;
  gap: 10px;
  width: 100%;
  padding: 9px 12px;
  background: transparent;
  border: none;
  border-radius: 0;
  color: var(--text);
  font-size: 13px;
  font-family: inherit;
  cursor: pointer;
  text-align: left;
  transition: background 0.1s ease;
}

.context-menu-item:hover:not(.disabled) {
  background: var(--hover-bg);
}

.context-menu-item.disabled {
  opacity: 0.45;
  cursor: not-allowed;
}

.context-menu-item i.material-icons {
  font-size: 18px;
  width: 22px;
  text-align: center;
  flex-shrink: 0;
}

.context-menu-item .shortcut-icon {
  margin-left: auto;
  font-size: 14px;
  color: var(--text-dim);
  opacity: 0.6;
}

.context-menu-item.danger {
  color: var(--status-error);
}

.context-menu-item.danger:hover {
  background: rgba(239, 83, 80, 0.12);
}

.context-menu-item.danger i {
  color: var(--status-error);
}
</style>
