<!-- Compact list view card for a single download — single-row, tight spacing -->
<template>
  <div class="download-list-card-compact" @contextmenu.prevent="openContextMenu" @dblclick="handleDoubleClick">
    <div class="dlc-row">
      <div class="dlc-main">
        <span class="dlc-title">{{ download.filename }}</span>
        <span class="dlc-meta">
          <span :style="getStatusTextStyle(download.status)" class="dlc-status-pill">
            <i class="material-icons">{{ getStatusIcon(download.status) }}</i>
            <span>{{ capitalizeStatus(download.status) }}</span>
          </span>
          <span v-if="download.source === 'yt-dlp'" class="dlc-source-tag dlc-source-media">
            <i class="material-icons" style="font-size:10px;">smart_display</i>
            {{ t('Media') }}
          </span>
          <span
            v-if="download.fileState === 'moved' || download.fileState === 'removed'"
            class="dlc-file-state"
            :class="download.fileState"
            :title="download.fileState === 'moved' ? t('File was moved from its recorded location') : t('File no longer exists')"
          >
            <i class="material-icons" style="font-size:10px;">{{ download.fileState === 'moved' ? 'drive_file_move' : 'delete' }}</i>
            {{ download.fileState === 'moved' ? t('Moved') : t('Removed') }}
          </span>
        </span>
      </div>

      <span class="dlc-size">{{ download.size }}</span>
      <span class="dlc-spacer" />

      <span class="dlc-speed" v-if="download.status === 'active'">
        <i class="material-icons">arrow_downward</i>
        <span class="dlc-speed-stack">
          <span>{{ download.speed }}</span>
          <span class="dlc-connections-pill" :title="connectionsTitle"><span class="dlc-connections-num">{{ displayedConnections }}</span><i class="material-icons">layers</i></span>
          <span v-if="download.remainingSeconds != null" class="dlc-eta">{{ formatRemainingTime(download.remainingSeconds) }}</span>
        </span>
      </span>
      <span v-else class="dlc-speed dlc-muted">—</span>

      <span class="dlc-spacer" />

      <div class="dlc-actions">
        <button v-if="download.status === 'active' || download.status === 'waiting'" @click="pauseDownload(download.gid)" class="dlc-btn" :title="t('Pause')"><i class="material-icons">pause</i></button>
        <button v-else-if="download.status === 'paused'" @click="resumeDownload(download.gid)" class="dlc-btn" :title="t('Resume')"><i class="material-icons">play_arrow</i></button>
        <button v-else-if="download.status === 'error'" @click="retryDownload(download)" class="dlc-btn" :title="t('Retry')"><i class="material-icons">refresh</i></button>
        <button v-else-if="download.status === 'complete'" @click="redownloadFile(download)" class="dlc-btn" :title="t('Redownload')"><i class="material-icons">download</i></button>
        <button v-else-if="download.status === 'cancelled'" @click="retryDownload(download)" class="dlc-btn" :title="t('Retry')"><i class="material-icons">refresh</i></button>
        <button @click="showInFolder(download)" class="dlc-btn" :title="t('Show in Folder')"><i class="material-icons">folder_open</i></button>
        <div class="group-card-btn-wrapper" v-if="groups.length > 0 && download.status !== 'complete'">
          <button class="dlc-btn dlc-group-btn" @click.stop="showGroupDropdown = !showGroupDropdown" :title="t('Add to Group')"><i class="material-icons">group_add</i></button>
        </div>
        <button v-if="download.status === 'active' || download.status === 'waiting' || download.status === 'paused'" @click="cancelDownload(download.gid)" class="dlc-btn dlc-btn-cancel" :title="t('Cancel')"><i class="material-icons">cancel</i></button>
        <button v-else @click="removeDownload(download.gid)" class="dlc-btn dlc-btn-del" :title="t('Remove')"><i class="material-icons">close</i></button>
      </div>
    </div>

    <!-- Progress bar at bottom -->
    <div class="dlc-progress-track"><div class="dlc-progress-fill" :class="{ experimental: download.engine === 'experimental' }" :style="{ width: download.progress + '%', background: download.status === 'error' ? '#ef5350' : (download.status === 'paused' ? '#d68e26' : (download.status === 'cancelled' ? '#9e9e9e' : (download.progress === 100 ? '#81c784' : 'var(--accent)'))) }"></div></div>

    <!-- Group dropdown -->
    <transition name="group-dropdown-fade">
      <div v-if="showGroupDropdown" class="group-dropdown-menu" style="position: absolute; bottom: 100%; right: 0; margin-bottom: 8px; z-index: 20;">
        <div v-if="groups.length === 0" class="group-dropdown-empty">{{ t('No groups yet') }}</div>
        <button v-for="group in groups" :key="group.id" class="group-dropdown-item" @click="addToGroup(group.id, download.gid); showGroupDropdown = false" :style="{ '--group-color': group.color }">
          <span class="group-dropdown-dot" :style="{ background: group.color }"></span>
          <span>{{ group.name }}</span>
        </button>
      </div>
    </transition>
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

const openContextMenuInjected = inject('openContextMenu') as ((download: Download, x: number, y: number) => void) | undefined

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

function openContextMenu(e: MouseEvent) {
  if (openContextMenuInjected) {
    openContextMenuInjected(props.download, e.clientX, e.clientY)
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
</script>

<style scoped>
/* Compact row layout */
.download-list-card-compact { position: relative; background: var(--bg-card); border-radius: 8px; padding: 6px 12px; border: 1px solid var(--border); display: flex; flex-direction: column; gap: 4px; cursor: pointer; transition: box-shadow 0.2s ease, transform 0.2s ease, border-color 0.2s ease; }
.download-list-card-compact:focus:not(:focus-visible) {
  outline: none !important;
  box-shadow: none !important;
  box-shadow: 0 0 0 0 transparent !important;
  -webkit-focus-ring-color: transparent !important;
}
.download-list-card-compact:focus-visible {
  outline: 2px solid var(--accent) !important;
  outline-offset: 2px !important;
  box-shadow: 0 0 0 4px rgba(var(--accent-rgb), 0.2) !important;
}
.download-list-card-compact:hover { transform: translateY(-1px); border-color: rgba(var(--accent-rgb), 0.18); }
.download-list-card-compact:active { transform: translateY(-1px) scale(0.985); box-shadow: 0 2px 8px rgba(0,0,0,0.12); }
.dlc-row { display: flex; align-items: center; gap: 8px; }
.dlc-main { flex: 1; min-width: 0; display: flex; flex-direction: column; gap: 2px; }
.dlc-title { font-size: 12.5px; font-weight: 500; color: var(--text); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
.dlc-meta { display: flex; align-items: center; gap: 6px; font-size: 10.5px; }
.dlc-status-pill { display: inline-flex; align-items: center; gap: 3px; font-size: 9.5px; font-weight: 600; padding: 1px 6px; border-radius: 999px; }
.dlc-status-pill .material-icons { font-size: 11px; }
.dlc-source-tag { display: inline-flex; align-items: center; gap: 2px; font-size: 8.5px; font-weight: 600; padding: 1px 5px; border-radius: 3px; letter-spacing: 0.03em; }
.dlc-source-media { background: rgba(var(--accent-rgb),0.12); color: var(--accent-strong); border: 1px solid rgba(var(--accent-rgb),0.2); }
.dlc-file-state { display: inline-flex; align-items: center; gap: 2px; font-size: 8.5px; font-weight: 700; padding: 1px 5px; border-radius: 3px; letter-spacing: 0.03em; text-transform: uppercase; }
.dlc-file-state .material-icons { font-size: 10px; }
.dlc-file-state.moved { background: rgba(255,167,38,0.12); color: var(--status-paused); border: 1px solid rgba(255,167,38,0.2); }
.dlc-file-state.removed { background: rgba(239,83,80,0.1); color: var(--status-error); border: 1px solid rgba(239,83,80,0.2); }
.dlc-size { font-size: 10.5px; color: var(--text-dim); white-space: nowrap; min-width: 80px; text-align: right; flex-shrink: 0; }
.dlc-spacer { flex: 0 0 4px; }
.dlc-speed { display: inline-flex; align-items: center; gap: 3px; font-size: 11px; font-weight: 500; color: var(--accent-strong); white-space: nowrap; min-width: 90px; flex-shrink: 0; }
.dlc-speed .material-icons { font-size: 12px; }
.dlc-muted { color: var(--text-dim); }
.dlc-speed-stack { display: inline-flex; align-items: center; gap: 4px; }
.dlc-connections-pill { display: inline-flex; align-items: center; justify-content: center; gap: 2px; background: rgba(var(--accent-rgb),0.06); color: var(--accent-strong); padding: 1px 5px; border-radius: 4px; font-size: 9.5px; font-weight: 600; line-height: 1; }
.dlc-connections-pill .dlc-connections-num { display: inline-flex; align-items: center; line-height: 1; }
.dlc-connections-pill .material-icons { font-size: 10px; line-height: 1; opacity: 0.95; }
.dlc-eta { font-size: 9.5px; color: var(--text-dim); font-weight: 600; line-height: 1; }
.dlc-muted { color: var(--text-dim); }
.dlc-actions { display: flex; align-items: center; gap: 2px; }
.dlc-btn { position: relative; display: flex; align-items: center; justify-content: center; width: 26px; height: 26px; border: none; background: none; color: var(--text-muted); border-radius: 5px; cursor: pointer; transition: background 0.12s, color 0.12s, transform 0.12s; flex-shrink: 0; }
.dlc-btn:focus:not(:focus-visible) {
  outline: none !important;
  box-shadow: none !important;
  -webkit-focus-ring-color: transparent !important;
}
.dlc-btn:hover { background: var(--hover-bg); color: var(--text); transform: scale(1.1); }
.dlc-btn .material-icons { font-size: 15px; }
.dlc-btn-cancel:hover { color: var(--status-paused); }
.dlc-btn-del:hover { color: var(--status-error); }
.dlc-progress-track { position: absolute; bottom: 0; left: 8px; right: 8px; height: 2px; background: var(--hover-bg); border-radius: 1px; overflow: hidden; }
.dlc-progress-fill { height: 100%; border-radius: 1px; transition: width 0.3s ease; }
.dlc-progress-fill.experimental { transition: none; }
.group-card-btn-wrapper { position: relative; }
.group-btn:hover { color: var(--accent-strong); }
.group-dropdown-menu { background: var(--bg-card); border: 1px solid var(--border); border-radius: 10px; box-shadow: var(--shadow-lg); padding: 6px 0; min-width: 160px; }
.group-dropdown-empty { font-size: 11px; color: var(--text-dim); padding: 6px 10px; }
.group-dropdown-item { display: flex; align-items: center; gap: 7px; width: 100%; background: none; border: none; color: var(--text); cursor: pointer; padding: 6px 10px; font-size: 12px; text-align: left; transition: background 0.1s; }
.group-dropdown-item:hover { background: rgba(var(--accent-rgb), 0.08); }
.group-dropdown-dot { width: 7px; height: 7px; border-radius: 50%; flex-shrink: 0; }
@keyframes group-dropdown-fade-enter { from { opacity: 0; transform: translateY(6px); } to { opacity: 1; transform: translateY(0); } }
@keyframes group-dropdown-fade-leave { from { opacity: 1; transform: translateY(0); } to { opacity: 0; transform: translateY(6px); } }
.group-dropdown-fade-enter-active, .group-dropdown-fade-leave-active { transition: all 0.12s ease; }
.group-dropdown-fade-enter-from, .group-dropdown-fade-leave-to { opacity: 0; transform: translateY(6px); }
</style>
