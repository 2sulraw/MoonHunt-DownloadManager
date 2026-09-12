<template>
  <div class="main-layout">
    <SideBar :active-route="route.path" @navigate="$router.push($event)" @about="showAboutModal = true" />

    <div class="main-content">
      <TitleBar
        :title="pageTitle"
        v-model:searchQuery="searchQuery"
        :searchEnabled="route.path === '/downloads'"
        :showDownloadActions="route.path === '/'"
        :update-available="updateAvailable"
        @new-download="handleNewDownload"
        @batch-download="handleBatchDownload"
        @update-available-click="showUpdateModal = true"
      />

      <div class="content-area">
        <router-view v-slot="{ Component }">
          <keep-alive>
            <component :is="Component" :downloads="downloads" @refresh="refreshDownloads" @fast-refresh="temporaryFastRefresh" @cancel="cancelDownload" @remove="removeDownload" />
          </keep-alive>
        </router-view>
      </div>
    </div>

    <DownloadConfirmDialog ref="downloadDialog" :downloads="downloads" />
    <DownloadLimitAlert ref="downloadLimitAlert" />

    <GlobalContextMenu
      ref="globalContextMenu"
      :download="contextMenuDownload"
      :visible="contextMenuVisible"
      :position="contextMenuPosition"
      :groups="contextMenuGroups"
      @action="handleGlobalContextAction"
      @add-to-group="handleAddToGroup"
      @close="closeGlobalContextMenu"
    />

    <ToastContainer />
    <ClipboardToast ref="clipboardToastRef" @add-download="handleClipboardDownload" />
    <AboutModal :visible="showAboutModal" @close="showAboutModal = false" @update-available="onUpdateAvailable" />
    <UpdateModal :visible="showUpdateModal" :update="updateInfo" @close="showUpdateModal = false" />
    <CloseDialog :visible="showCloseDialog" @close="showCloseDialog = false" />
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, provide } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { upsertDownload } from '../db'
import { useNotification } from '../composables/useNotification'
import { useTheme } from '../composables/useTheme'
import { useDownloads } from '../composables/useDownloads'
import { useGroups } from '../composables/useGroups'
import { useDownloadLimit } from '../composables/useDownloadLimit'
import { useAria2Startup } from '../composables/useAria2Startup'
import type { Download } from '../types/download'
import type { DownloadGroup } from '../composables/useGroups'
import { useI18n } from '../i18n'
import TitleBar from '../components/TitleBar.vue'
import SideBar from '../components/SideBar.vue'
import ToastContainer from '../components/ToastContainer.vue'
import AboutModal from '../components/AboutModal.vue'
import UpdateModal from '../components/UpdateModal.vue'
import CloseDialog from '../components/CloseDialog.vue'
import ClipboardToast from '../components/ClipboardToast.vue'
import DownloadConfirmDialog from '../components/DownloadConfirmDialog.vue'
import DownloadLimitAlert from '../components/DownloadLimitAlert.vue'
import GlobalContextMenu from '../components/GlobalContextMenu.vue'

const route = useRoute()
const router = useRouter()
const { t } = useI18n()
const { notify } = useNotification()
const { loadTheme } = useTheme()
const { startAria2, retryAria2 } = useAria2Startup()
const {
  downloads, refreshDownloads, cancelDownload, removeDownload, temporaryFastRefresh,
  startPolling, rememberRequestedConnections, handleYtDlpStart,
} = useDownloads()
const { groups, reloadGroups, addMember, reapplyAllLimits } = useGroups()
const { onLimitReached, loadLimitSettings, resetLimit, startPeriodicPersist, stopPeriodicPersist } = useDownloadLimit()

const showAboutModal = ref(false)
const showUpdateModal = ref(false)
const updateAvailable = ref(false)
const updateInfo = ref<{ version?: string; url?: string; assetUrl?: string; body?: string } | null>(null)

function onUpdateAvailable(info: { version?: string; url?: string; assetUrl?: string; body?: string }) {
  updateInfo.value = info
  showAboutModal.value = false
  showUpdateModal.value = true
}
const showCloseDialog = ref(false)
const contextMenuVisible = ref(false)
const contextMenuPosition = ref({ x: 0, y: 0 })
const contextMenuDownload = ref<Download | null>(null)
const contextMenuGroups = ref<DownloadGroup[]>([])
const globalContextMenu = ref<InstanceType<typeof GlobalContextMenu> | null>(null)
const downloadDialog = ref<InstanceType<typeof DownloadConfirmDialog> | null>(null)
const downloadLimitAlert = ref<InstanceType<typeof DownloadLimitAlert> | null>(null)
const clipboardToastRef = ref<InstanceType<typeof ClipboardToast> | null>(null)

const pageTitle = computed(() => {
  const titles: Record<string, string> = { '/': t('Dashboard'), '/downloads': t('Downloads'), '/media': t('Media'), '/settings': t('Settings') }
  return titles[route.path] || 'MoonHunt'
})

provide('openContextMenu', openGlobalContextMenu)
provide('notify', notify)
provide('rememberRequestedConnections', rememberRequestedConnections)

const searchQuery = ref('')
provide('searchQuery', searchQuery)

function openGlobalContextMenu(download: Download, x: number, y: number) {
  contextMenuDownload.value = download
  contextMenuPosition.value = { x, y }
  reloadGroups()
  contextMenuGroups.value = groups.value
  contextMenuVisible.value = true
}

function closeGlobalContextMenu() {
  contextMenuVisible.value = false
  contextMenuDownload.value = null
}

async function handleAddToGroup(groupId: string, download: Download) {
  if (download.status === 'complete') return
  const g = await addMember(groupId, download.gid)
  if (g) {
    notify(`${t('Added to')}: "${g.name}"`, 'success')
    window.dispatchEvent(new Event('moonhunt:groups-changed'))
  }
}

async function handleGlobalContextAction(action: string, download: Download) {
  const isYtDirect = download.gid.startsWith('yt-dlp-direct-')
  switch (action) {
    case 'pause':
      if (isYtDirect) {
        invoke('cancel_yt_download', { gid: download.gid }).then(() => refreshDownloads()).catch(e => console.error(e))
      } else {
        invoke('pause_download', { gid: download.gid }).then(() => refreshDownloads()).catch(e => console.error(e))
      }
      break
    case 'resume':
      if (isYtDirect) {
        // Media downloads can't be resumed mid-file; re-run the source URL.
        invoke('resume_yt_download', { gid: download.gid }).then(() => refreshDownloads()).catch(e => console.error(e))
      } else {
        invoke('resume_download', { gid: download.gid }).then(() => refreshDownloads()).catch(e => console.error(e))
      }
      break
    case 'retry':
      if (isYtDirect) {
        // Media downloads can't be resumed from aria2; cancel the old row and
        // re-run the source URL from its stored record.
        invoke('cancel_yt_download', { gid: download.gid }).then(() => {
          if (download.url && /^https?:\/\//i.test(download.url)) {
            return invoke('add_yt_download', {
              url: download.url, format: 'mp4', quality: 'best',
              path: download.path ? download.path.split(/[\\/]/).slice(0, -1).join('/') : undefined,
              browser: undefined, format_id: undefined,
            })
          }
        }).then(() => refreshDownloads()).catch(e => console.error(e))
      } else {
        invoke('retry_download', { gid: download.gid }).catch(e => console.error(e))
      }
      break
    case 'redownload':
      rememberRequestedConnections(download.url, download.connections ?? 1)
      invoke('add_download', {
      url: download.url, filename: download.filename,
      connections: download.connections ?? 1,
      dir: download.path ? download.path.split(/[\\/]/).slice(0, -1).join('/') : undefined,
      proxy: download.proxy || null,
      }).catch(e => console.error(e)); break
    case 'showInFolder':
      invoke('show_download_in_folder', { gid: download.gid, filePath: download.path || null, filename: download.filename || null })
        .catch((e: unknown) => notify(t('Failed to open folder') + ': ' + e, 'error'))
      break
    case 'clearError':
      try {
        await upsertDownload({ ...download, status: 'complete', progress: 100, errorCode: '', errorMessage: '' })
        await refreshDownloads()
        notify(t('Error cleared'), 'success')
      } catch (e) { console.error('Failed to clear error:', e) }
      break
    case 'details': window.dispatchEvent(new CustomEvent('moonhunt:show-details', { detail: download })); break
    case 'verifyHash': window.dispatchEvent(new CustomEvent('moonhunt:verify-hash', { detail: { filePath: download.path || '', filename: download.filename } })); break
    case 'copyUrl': navigator.clipboard.writeText(download.url).then(() => notify(t('URL copied'), 'success')).catch(() => {}); break
    case 'copyFilename': navigator.clipboard.writeText(download.filename).then(() => notify(t('Filename copied'), 'success')).catch(() => {}); break
    case 'cancel': cancelDownload(download.gid).then(() => notify(t('Download cancelled'), 'success')).catch(e => console.error(e)); break
    case 'remove': removeDownloadWithNotify(download.gid); break
  }
}

onLimitReached.value = async (used: number, limit: number) => {
  if (!downloadLimitAlert.value) return
  const result = await downloadLimitAlert.value.open(used, limit)
  if (result?.reset) {
    resetLimit()
    notify(t('Download limit reset. Downloads remain paused.'), 'success')
  }
}

let stopPollingCleanup: (() => void) | null = null
let pendingCheckInterval: number | null = null
let pendingDownloadUnlisten: (() => void) | null = null
let scheduleCheckInterval: number | null = null
let clipboardUnlisten: (() => void) | null = null
let closeRequestUnlisten: (() => void) | null = null
let torrentDropUnlisten: Array<() => void> = []

/**
 * Capture OS file drags at the window level. When .torrent files are dropped
 * anywhere on the app, dispatch moonhunt:torrent-drop so the Downloads page
 * opens the New Download dialog on the Torrent tab with the files preloaded.
 */
function setupWindowTorrentDrop() {
  import('@tauri-apps/api/event').then(({ listen }) => {
    Promise.all([
      listen<{ paths?: string[] }>('tauri://drag-drop', (e) => {
        const paths = e.payload?.paths || []
        const torrents = paths.filter((p) => p.toLowerCase().endsWith('.torrent'))
        if (!torrents.length) return
        window.dispatchEvent(new CustomEvent('moonhunt:torrent-drop', { detail: { paths: torrents } }))
      }),
    ]).then((unlistens) => { torrentDropUnlisten = unlistens }).catch(() => {})
  })
}

async function handleClipboardDownload(url: string) {
  if (!downloadDialog.value) return
  try {
    const result = await downloadDialog.value.openDialog(url, '', '', undefined)
    if (result?.accepted) {
      await invoke<string>('confirm_download_action', {
        url, filename: result.filename || '',
        connections: result.connections || 1,
        dir: result.filePath || undefined,
        proxy: result.proxy || null,
      })
      notify(`${t('Download added')}: ${result.filename || url}`, 'success')
    }
  } catch (e) {
    notify(`${t('Failed to start download')}: ${e}`, 'error')
  }
}

function handleNewDownload() {
  if (route.path !== '/downloads') {
    router.push('/downloads').then(() => {
      setTimeout(() => window.dispatchEvent(new CustomEvent('moonhunt:new-download')), 100)
    })
  } else {
    window.dispatchEvent(new CustomEvent('moonhunt:new-download'))
  }
}

function handleBatchDownload() {
  if (route.path !== '/downloads') {
    router.push('/downloads').then(() => {
      setTimeout(() => window.dispatchEvent(new CustomEvent('moonhunt:batch-download')), 100)
    })
  } else {
    window.dispatchEvent(new CustomEvent('moonhunt:batch-download'))
  }
}

function startClipboardListener() {
  invoke('start_clipboard_listener').catch(() => {})
  import('@tauri-apps/api/event').then(({ listen }) => {
    listen<string>('clipboard-url-detected', (event) => {
      if (clipboardToastRef.value) {
        clipboardToastRef.value.show(event.payload)
      }
    }).then((unlisten) => { clipboardUnlisten = unlisten })
  })
}

function onYtDlpStartEvent(e: Event) {
  const ce = e as CustomEvent
  handleYtDlpStart({ payload: ce?.detail, url: ce?.detail?.url })
}
window.addEventListener('yt-dlp-start', onYtDlpStartEvent)

async function removeDownloadWithNotify(gid: string) {
  try {
    await removeDownload(gid)
    notify(t('Download removed successfully'), 'success')
  } catch (error) {
    console.error('Failed to remove download:', error)
    notify(t('Failed to remove download') + ': ' + String(error), 'error')
  }
}

let processingPending = false

const checkPendingDownloads = async () => {
  if (!downloadDialog.value) return
  // Don't re-open dialog if it's already showing or being processed
  if (downloadDialog.value.isDialogOpen() || processingPending) return
  try {
    const pending = await invoke<Array<{ url: string; filename: string; headers?: Record<string, string> }>>('get_pending_downloads')
    if (pending && pending.length > 0) {
      const dl = pending[0]
      const result = await downloadDialog.value.openDialog(dl.url, dl.filename, '', dl.headers)
      if (result?.accepted) {
        processingPending = true
        rememberRequestedConnections(dl.url, result.connections || 1)
        try {
          const gid = await invoke<string>('confirm_download_action', {
            url: dl.url, filename: result.filename || dl.filename,
            connections: result.connections || 1, dir: result.filePath || undefined,
            headers: dl.headers || undefined,
            proxy: result.proxy || null,
          })
          if (result.schedule) {
            try {
              await invoke('create_schedule', {
                schedule: { downloadGid: gid || null, url: dl.url, filename: result.filename || dl.filename, ...result.schedule }
              })
            } catch { /* schedule failed but download added */ }
          }
          notify(`${t('Download added')}: ${result.filename || dl.filename}`, 'success')
        } catch (e) {
          notify(`${t('Failed to start download')}: ${e}`, 'error')
        }
      }
      await invoke('remove_pending_download', { index: 0 })
      processingPending = false
    }
  } catch (error) {
    processingPending = false
    console.error('Error checking pending downloads:', error)
  }
}

const checkDueSchedules = async () => {
  try {
    const dueSchedules = await invoke<Array<{ id: number; action: string; filename: string }>>('get_due_schedules')
    if (dueSchedules && dueSchedules.length > 0) {
      for (const schedule of dueSchedules) {
        try { await invoke('execute_schedule', { scheduleId: schedule.id }) } catch { /* skip */ }
      }
      await refreshDownloads()
    }
  } catch (error) {
    console.error('Error checking schedules:', error)
  }
}

onMounted(async () => {
  await loadTheme()
  await loadLimitSettings()
  startPeriodicPersist()

  ;(async () => {
    const started = await startAria2()
    await refreshDownloads()
    stopPollingCleanup = startPolling()
    await reapplyAllLimits()
    if (!started) await retryAria2()
  })()

  // One-shot update check on launch; badge appears in the titlebar if newer.
  invoke<{ status: string; version?: string; url?: string; assetUrl?: string; body?: string }>('check_for_update').then((res) => {
    if (res.status === 'available') {
      updateAvailable.value = true
      updateInfo.value = { version: res.version, url: res.url, assetUrl: res.assetUrl, body: res.body }
    }
  }).catch(() => {})

  // Pending downloads are now pushed from the backend. Keep a slow fallback
  // poll in case an event is missed before the listener is registered.
  pendingCheckInterval = window.setInterval(checkPendingDownloads, 5000)
  listen('pending-download', () => {
    void checkPendingDownloads()
  }).then((unlisten) => { pendingDownloadUnlisten = unlisten }).catch(() => {})
  scheduleCheckInterval = window.setInterval(checkDueSchedules, 10000)
  startClipboardListener()
  setupWindowTorrentDrop()
  listen('close-requested', () => {
    showCloseDialog.value = true
  }).then((unlisten) => { closeRequestUnlisten = unlisten }).catch(() => {})
})

onUnmounted(() => {
  if (stopPollingCleanup) stopPollingCleanup()
  if (pendingCheckInterval) clearInterval(pendingCheckInterval)
  if (pendingDownloadUnlisten) pendingDownloadUnlisten()
  if (scheduleCheckInterval) clearInterval(scheduleCheckInterval)
  if (clipboardUnlisten) clipboardUnlisten()
  if (closeRequestUnlisten) closeRequestUnlisten()
  torrentDropUnlisten.forEach((fn) => { try { fn() } catch { /* already removed */ } })
  torrentDropUnlisten = []
  stopPeriodicPersist()
  window.removeEventListener('yt-dlp-start', onYtDlpStartEvent)
})
</script>

<style>
.main-layout {
  display: flex;
  height: 100vh;
  overflow: hidden;
}

.main-content {
  flex: 1;
  display: flex;
  flex-direction: column;
  background: var(--bg-surface);
  overflow: hidden;
}

.content-area {
  padding: 18px 20px 24px;
  overflow-y: auto;
  flex: 1;
  background: var(--bg-surface);
}

/* Transition classes for modal-fade */
.modal-fade-enter-active { transition: opacity 0.2s ease; }
.modal-fade-leave-active { transition: opacity 0.15s ease; }
.modal-fade-enter-from, .modal-fade-leave-to { opacity: 0; }
</style>
