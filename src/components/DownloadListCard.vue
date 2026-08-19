<!-- List view card for a single download — horizontal, data-dense, modern -->
<template>
  <div class="download-list-card" @contextmenu.prevent="_openContextMenu" @dblclick="handleDoubleClick">
    <div class="dlc-body">

      <!-- Top row: status dot + filename + quick actions -->
      <div class="dlc-top">
        <div class="dlc-title-area">
          <span class="dlc-filename" :title="download.filename">{{ download.filename }}</span>
        </div>
        <div class="dlc-actions">
          <button v-if="download.status === 'active' || download.status === 'waiting'" @click="pauseDownload(download.gid)" class="dlc-btn" :title="t('Pause')">
            <i class="material-icons">pause</i>
          </button>
          <button v-else-if="download.status === 'paused'" @click="resumeDownload(download.gid)" class="dlc-btn" :title="t('Resume')">
            <i class="material-icons">play_arrow</i>
          </button>
          <button v-else-if="download.status === 'error'" @click="retryDownload(download)" class="dlc-btn" :title="t('Retry')">
            <i class="material-icons">refresh</i>
          </button>
          <button v-else-if="download.status === 'complete'" @click="redownloadFile(download)" class="dlc-btn" :title="t('Redownload')">
            <i class="material-icons">download</i>
          </button>
          <button v-else-if="download.status === 'cancelled'" @click="retryDownload(download)" class="dlc-btn" :title="t('Retry')">
            <i class="material-icons">refresh</i>
          </button>
          <button @click="showInFolder(download)" class="dlc-btn" :title="t('Show in Folder')">
            <i class="material-icons">folder_open</i>
          </button>
          <button @click="showDownloadDetails(download)" class="dlc-btn" :title="t('Details')">
            <i class="material-icons">description</i>
          </button>
          <button v-if="download.status === 'active' || download.status === 'waiting' || download.status === 'paused'" @click="cancelDownload(download.gid)" class="dlc-btn dlc-btn-cancel" :title="t('Cancel')">
            <i class="material-icons">cancel</i>
          </button>
          <button v-else @click="removeDownload(download.gid)" class="dlc-btn dlc-btn-remove" :title="t('Remove')">
            <i class="material-icons">close</i>
          </button>
        </div>
      </div>

      <!-- Info row: pills + meta -->
      <div class="dlc-info">
        <span class="dlc-pill" :style="{ background: statusBg, color: statusColor }">
          <i class="material-icons">{{ getStatusIcon(download.status) }}</i>
          <span>{{ capitalizeStatus(download.status) }}</span>
        </span>
        <span v-if="download.source === 'yt-dlp'" class="dlc-badge yt">
          <i class="material-icons">smart_display</i> yt-dlp
        </span>
        <span v-else class="dlc-badge http">HTTP</span>
        <span
          v-if="download.fileState === 'moved' || download.fileState === 'removed'"
          class="dlc-file-state"
          :class="download.fileState"
          :title="download.fileState === 'moved' ? t('File was moved from its recorded location') : t('File no longer exists')"
        >
          <i class="material-icons">{{ download.fileState === 'moved' ? 'drive_file_move' : 'delete' }}</i>
          {{ download.fileState === 'moved' ? t('Moved') : t('Removed') }}
        </span>
        <span class="dlc-size">{{ download.size }}</span>
        <span v-if="download.speed && download.status === 'active'" class="dlc-speed">
          <i class="material-icons">arrow_downward</i>
          {{ download.speed }}
        </span>
        <span v-if="download.status === 'active' && download.remainingSeconds != null" class="dlc-eta">
          <i class="material-icons">schedule</i>
          {{ formatRemainingTime(download.remainingSeconds) }}
        </span>
      </div>

      <!-- Progress bar -->
      <div class="dlc-progress-track">
        <div class="dlc-progress-fill" :style="progressStyle"></div>
      </div>

      <!-- Bottom row -->
      <div class="dlc-bottom">
        <div class="dlc-bottom-left">
          <span class="dlc-connections" :title="connectionsTitle">
            <i class="material-icons">layers</i>
            {{ displayedConnections }} {{ t('connections') }}
          </span>
          <button v-if="download.status === 'error' && (download.errorMessage || download.errorCode)" @click="showErrorDetails(download)" class="dlc-btn-sm error-sm">
            <i class="material-icons">receipt_long</i> {{ t('Error') }}
          </button>
        </div>
        <div class="dlc-bottom-right">
          <div class="dlc-group-wrap" v-if="groups.length > 0 && download.status !== 'complete'">
            <button class="dlc-btn-sm" @click.stop="showGroupDropdown = !showGroupDropdown">
              <i class="material-icons">group_add</i> {{ t('Group') }}
            </button>
            <transition name="dlc-drop">
              <div v-if="showGroupDropdown" class="dlc-dropdown" @click.stop>
                <div v-if="groups.length === 0" class="dlc-drop-empty">{{ t('No groups') }}</div>
                <button v-for="group in groups" :key="group.id" class="dlc-drop-item" @click="addToGroup(group.id, download.gid); showGroupDropdown = false">
                  <span class="dlc-drop-dot" :style="{ background: group.color }"></span>
                  <span>{{ group.name }}</span>
                </button>
              </div>
            </transition>
          </div>
        </div>
      </div>

    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, inject, computed } from 'vue'
import type { Download } from '../types/download'
import { formatRemainingTime } from '../utils/format'
import { useI18n } from '../i18n'

const { t } = useI18n()

interface Props {
  download: Download
  getRequestedConnections: (download: Download) => number
  getStatusTextStyle: (status: string) => string
  getStatusIcon: (status: string) => string
  capitalizeStatus: (status: string) => string
  showDownloadDetails: (download: Download) => void
  showErrorDetails: (download: Download) => void
  pauseDownload: (gid: string) => void
  resumeDownload: (gid: string) => void
  retryDownload: (download: Download) => void
  redownloadFile: (download: Download) => void
  showInFolder: (download: Download) => void
  openDownloadFile: (download: Download) => void
  cancelDownload: (gid: string) => void
  removeDownload: (gid: string) => void
  groups: { id: string; name: string; memberGids: string[]; color: string; collapsed: boolean }[]
  addToGroup: (groupId: string, gid: string) => void
}

const props = defineProps<Props>()

const openContextMenu = inject('openContextMenu') as ((download: Download, x: number, y: number) => void) | undefined
const showGroupDropdown = ref(false)

const displayedConnections = computed(() => {
  const d = props.download
  if (d.status === 'active' || d.status === 'waiting') {
    return d.activeConnections ?? d.connections ?? 1
  }
  return d.connections ?? 1
})

const connectionsTitle = computed(() => {
  const live = displayedConnections.value
  const requested = props.getRequestedConnections(props.download)
  return live === requested
    ? `${live} ${t('connections')}`
    : `${live} ${t('active')} / ${requested} ${t('requested')}`
})

function _openContextMenu(e: MouseEvent) {
  if (openContextMenu) {
    openContextMenu(props.download, e.clientX, e.clientY)
  }
}

function handleDoubleClick(e: MouseEvent) {
  if ((e.target as HTMLElement).closest('button, a, input, select, textarea')) return
  const d = props.download
  if (d.status === 'complete') {
    props.openDownloadFile(d)
  } else if (d.status === 'active' || d.status === 'waiting') {
    props.pauseDownload(d.gid)
  } else if (d.status === 'paused') {
    props.resumeDownload(d.gid)
  }
}

// ── Status styling ──
const statusBg = computed(() => {
  const s = props.download.status
  if (s === 'active') return 'rgba(79,195,247,0.12)'
  if (s === 'complete') return 'rgba(129,199,132,0.12)'
  if (s === 'paused') return 'rgba(255,167,38,0.12)'
  if (s === 'error') return 'rgba(239,83,80,0.12)'
  if (s === 'waiting') return 'rgba(100,181,246,0.12)'
  if (s === 'cancelled') return 'rgba(158,158,158,0.12)'
  return 'rgba(105,112,138,0.12)'
})

const statusColor = computed(() => {
  const s = props.download.status
  if (s === 'active') return 'var(--status-active)'
  if (s === 'complete') return 'var(--status-complete)'
  if (s === 'paused') return 'var(--status-paused)'
  if (s === 'error') return 'var(--status-error)'
  if (s === 'waiting') return 'var(--status-waiting)'
  if (s === 'cancelled') return 'var(--status-cancelled)'
  return 'var(--text-dim)'
})

const progressStyle = computed(() => {
  const d = props.download
  let shadow = 'none'
  if (d.source === 'yt-dlp') {
    if (d.status === 'error') shadow = '0 0 6px rgba(239,83,80,0.6)'
    else if (d.status === 'paused') shadow = '0 0 6px rgba(214,142,38,0.6)'
    else if (d.progress === 100) shadow = '0 0 6px rgba(129,199,132,0.6)'
    else shadow = '0 0 6px rgba(var(--accent-rgb),0.6)'
  }
  return {
    width: d.progress + '%',
    height: '100%',
    background: d.status === 'error' ? '#ef5350' : (d.status === 'paused' ? '#d68e26' : (d.status === 'cancelled' ? '#9e9e9e' : (d.progress === 100 ? '#81c784' : 'var(--accent)'))),
        borderRadius: '4px',
    transition: 'width 0.4s ease',
    boxShadow: shadow,
  }
})
</script>

<style scoped>
/* ── Card container ── */
.download-list-card {
  position: relative;
  background: var(--bg-card);
  border-radius: 12px;
  padding: 12px 16px;
  border: 1px solid var(--border);
  display: flex;
  flex-direction: column;
  cursor: pointer;
  transition: all 0.2s ease;
}
.download-list-card:focus:not(:focus-visible) {
  outline: none !important;
  box-shadow: none !important;
  box-shadow: 0 0 0 0 transparent !important;
  -webkit-focus-ring-color: transparent !important;
}
.download-list-card:focus-visible {
  outline: 2px solid var(--accent) !important;
  outline-offset: 2px !important;
  box-shadow: 0 0 0 4px rgba(var(--accent-rgb), 0.2) !important;
}
.download-list-card:hover {
  border-color: rgba(var(--accent-rgb), 0.18);
  transform: translateY(-1px);
  box-shadow: 0 4px 16px rgba(0,0,0,0.1);
}
.download-list-card:active {
  transform: translateY(-1px) scale(0.985);
  box-shadow: 0 2px 8px rgba(0,0,0,0.12);
}
.dlc-body {
  display: flex;
  flex-direction: column;
  gap: 7px;
}

/* ── Top row ── */
.dlc-top {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
}
.dlc-title-area {
  display: flex;
  align-items: center;
  gap: 8px;
  min-width: 0;
  flex: 1;
}
.dlc-filename {
  font-size: 13.5px;
  font-weight: 600;
  color: var(--text);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  line-height: 1.3;
}
.dlc-actions {
  display: flex;
  align-items: center;
  gap: 1px;
  flex-shrink: 0;
}
.dlc-btn {
  background: none;
  border: none;
  color: var(--text-muted);
  width: 28px;
  height: 28px;
  border-radius: 6px;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: all 0.15s ease;
}
.dlc-btn:focus:not(:focus-visible) {
  outline: none !important;
  box-shadow: none !important;
  -webkit-focus-ring-color: transparent !important;
}
.dlc-btn i { font-size: 17px; }
.dlc-btn:hover { background: var(--hover-bg); color: var(--text); transform: scale(1.05); }
.dlc-btn-cancel:hover { color: var(--status-paused); }
.dlc-btn-remove:hover { color: var(--status-error); }

/* ── Info row ── */
.dlc-info {
  display: flex;
  align-items: center;
  gap: 6px;
  flex-wrap: wrap;
}
.dlc-pill {
  display: inline-flex;
  align-items: center;
  gap: 3px;
  font-size: 10.5px;
  font-weight: 700;
  padding: 2px 8px;
  border-radius: 100px;
  text-transform: uppercase;
  letter-spacing: 0.03em;
}
.dlc-pill i { font-size: 13px; }
.dlc-badge {
  display: inline-flex;
  align-items: center;
  gap: 3px;
  font-size: 10px;
  font-weight: 600;
  padding: 2px 7px;
  border-radius: 4px;
}
.dlc-badge.yt { background: rgba(239,83,80,0.08); color: var(--status-error); }
.dlc-badge.http { background: rgba(var(--accent-rgb),0.08); color: var(--accent-strong); }
.dlc-badge i { font-size: 12px; }
.dlc-file-state {
  display: inline-flex;
  align-items: center;
  gap: 3px;
  font-size: 10px;
  font-weight: 700;
  padding: 2px 7px;
  border-radius: 4px;
  text-transform: uppercase;
  letter-spacing: 0.03em;
}
.dlc-file-state i { font-size: 12px; }
.dlc-file-state.moved { background: rgba(255,167,38,0.12); color: var(--status-paused); }
.dlc-file-state.removed { background: rgba(239,83,80,0.1); color: var(--status-error); }
.dlc-size {
  font-size: 11px;
  color: var(--text-dim);
}
.dlc-speed {
  display: inline-flex;
  align-items: center;
  gap: 3px;
  font-size: 11px;
  font-weight: 600;
  color: var(--accent-strong);
}
.dlc-speed i { font-size: 14px; }
.dlc-eta {
  display: inline-flex;
  align-items: center;
  gap: 3px;
  font-size: 11px;
  font-weight: 600;
  color: var(--text-dim);
}
.dlc-eta i { font-size: 14px; }

/* ── Progress ── */
.dlc-progress-track {
  height: 5px;
  background: var(--hover-bg);
  border-radius: 4px;
  overflow: hidden;
}
.dlc-progress-fill {
  height: 100%;
  border-radius: 4px;
  transition: width 0.4s ease;
}

/* ── Bottom row ── */
.dlc-bottom {
  display: flex;
  align-items: center;
  justify-content: space-between;
}
.dlc-bottom-left {
  display: flex;
  align-items: center;
  gap: 8px;
}
.dlc-bottom-right {
  display: flex;
  align-items: center;
  gap: 4px;
}
.dlc-connections {
  display: inline-flex;
  align-items: center;
  gap: 3px;
  font-size: 10.5px;
  color: var(--text-muted);
}
.dlc-connections i { font-size: 12px; }
.dlc-btn-sm {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  background: var(--hover-bg);
  border: none;
  border-radius: 6px;
  color: var(--text-muted);
  font-size: 10.5px;
  font-weight: 600;
  padding: 4px 8px;
  cursor: pointer;
  transition: all 0.15s ease;
}
.dlc-btn-sm i { font-size: 13px; }
.dlc-btn-sm:hover { background: rgba(var(--accent-rgb), 0.08); color: var(--accent-strong); }
.error-sm:hover { background: rgba(239,83,80,0.1) !important; color: var(--status-error) !important; }

/* ── Group dropdown ── */
.dlc-group-wrap { position: relative; }
.dlc-dropdown {
  position: absolute;
  bottom: calc(100% + 6px);
  right: 0;
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: 10px;
  box-shadow: 0 8px 32px rgba(0,0,0,0.2);
  padding: 6px 0;
  min-width: 150px;
  z-index: 30;
}
.dlc-drop-empty {
  font-size: 12px;
  color: var(--text-dim);
  padding: 8px 12px;
}
.dlc-drop-item {
  display: flex;
  align-items: center;
  gap: 8px;
  width: 100%;
  background: none;
  border: none;
  color: var(--text);
  cursor: pointer;
  padding: 7px 12px;
  font-size: 12.5px;
  text-align: left;
  transition: background 0.1s;
}
.dlc-drop-item:hover { background: rgba(var(--accent-rgb), 0.08); }
.dlc-drop-dot { width: 7px; height: 7px; border-radius: 50%; flex-shrink: 0; }

/* Dropdown animation */
.dlc-drop-enter-active { animation: dlc-drop-in 0.12s ease; }
.dlc-drop-leave-active { animation: dlc-drop-in 0.1s ease reverse; }
@keyframes dlc-drop-in {
  from { opacity: 0; transform: translateY(6px); }
  to { opacity: 1; transform: translateY(0); }
}
</style>
