<template>
  <div class="home-page">
    <!-- Stats Bar -->
    <div class="stats-bar">
      <div class="stat-chip">
        <div class="chip-icon chip-blue"><i class="material-icons">play_circle</i></div>
        <div class="chip-info">
          <span class="chip-val">{{ activeCount }}</span>
          <span class="chip-label">{{ t('Active') }}</span>
        </div>
      </div>
      <div class="stat-chip">
        <div class="chip-icon chip-green"><i class="material-icons">check_circle</i></div>
        <div class="chip-info">
          <span class="chip-val">{{ completedCount }}</span>
          <span class="chip-label">{{ t('Completed') }}</span>
        </div>
      </div>
      <div class="stat-chip">
        <div class="chip-icon chip-red"><i class="material-icons">error</i></div>
        <div class="chip-info">
          <span class="chip-val">{{ failedCount }}</span>
          <span class="chip-label">{{ t('Failed') }}</span>
        </div>
      </div>
      <div class="stat-chip">
        <div class="chip-icon chip-yellow"><i class="material-icons">pause_circle</i></div>
        <div class="chip-info">
          <span class="chip-val">{{ pausedCount }}</span>
          <span class="chip-label">{{ t('Paused') }}</span>
        </div>
      </div>
      <div class="stat-chip">
        <div class="chip-icon chip-purple"><i class="material-icons">folder_open</i></div>
        <div class="chip-info">
          <span class="chip-val">{{ totalSizeText }}</span>
          <span class="chip-label">{{ t('Total Size') }}</span>
        </div>
      </div>
      <div class="stat-chip">
        <div class="chip-icon chip-teal"><i class="material-icons">today</i></div>
        <div class="chip-info">
          <span class="chip-val">{{ todayCount }}</span>
          <span class="chip-label">{{ t('Today') }}</span>
        </div>
      </div>
    </div>

    <!-- Speed Display -->
    <section class="speed-card">
      <div class="speed-main">
        <div class="speed-row">
          <div class="speed-item">
            <i class="material-icons speed-arrow">arrow_downward</i>
            <span class="speed-value">{{ totalDownloadSpeed }}</span>
            <span class="speed-label">{{ t('Download') }}</span>
          </div>
          <div class="speed-divider"></div>
          <div class="speed-item">
            <i class="material-icons speed-arrow">arrow_upward</i>
            <span class="speed-value">{{ totalUploadSpeed }}</span>
            <span class="speed-label">{{ t('Upload') }}</span>
          </div>
        </div>
        <p class="speed-sub">{{ activeCount }} {{ activeCount !== 1 ? t('active downloads') : t('active download') }}</p>
      </div>
      <div class="speed-visual">
        <div class="speed-bars">
          <div v-for="(h, i) in speedHistory" :key="i" class="speed-bar" :style="{ height: h + '%' }"></div>
        </div>
      </div>
    </section>

    <!-- Active Downloads -->
    <section class="panel">
      <div class="panel-header">
        <h2 class="panel-title">
          <i class="material-icons">download</i>
          {{ t('Active Downloads') }}
        </h2>
        <router-link to="/downloads" class="panel-link">{{ t('View All') }} <i class="material-icons">arrow_forward</i></router-link>
      </div>
      <div v-if="activeDownloads.length > 0" class="active-list">
        <div v-for="d in activeDownloads.slice(0, 5)" :key="d.gid" class="active-item">
          <div class="active-info">
            <span class="active-name">{{ d.filename }}</span>
            <span class="active-meta">{{ d.speed || '0 B/s' }} · {{ d.size }}<template v-if="d.remainingSeconds != null"> · {{ formatRemainingTime(d.remainingSeconds) }}</template></span>
          </div>
          <div class="active-progress">
            <div class="progress-track">
              <div class="progress-fill" :style="{ width: d.progress + '%' }"></div>
            </div>
            <span class="progress-pct">{{ d.progress }}%</span>
          </div>
        </div>
      </div>
      <div v-else class="empty-state">
        <i class="material-icons">cloud_download</i>
        <p>{{ t('No active downloads') }}</p>
        <span>{{ t('Start one from the Downloads page.') }}</span>
      </div>
    </section>

    <!-- Recent Activity -->
    <section class="panel">
      <div class="panel-header">
        <h2 class="panel-title">
          <i class="material-icons">history</i>
          {{ t('Recent Activity') }}
        </h2>
      </div>
      <div v-if="recentActivity.length > 0" class="activity-list">
        <div v-for="d in recentActivity" :key="d.gid" class="activity-item">
          <div class="activity-icon" :class="d.status === 'error' ? 'act-error' : 'act-success'">
            <i class="material-icons">{{ d.status === 'error' ? 'error' : 'check_circle' }}</i>
          </div>
          <div class="activity-info">
            <span class="activity-name">{{ d.filename }}</span>
            <span class="activity-meta">{{ formatSize(d.size) }} · {{ d.source || 'aria2' }}</span>
          </div>
          <span class="activity-time">{{ formatTime(d.added_at) }}</span>
        </div>
      </div>
      <div v-else class="empty-state">
        <i class="material-icons">hourglass_empty</i>
        <p>{{ t('No recent activity') }}</p>
        <span>{{ t('Completed and failed downloads appear here.') }}</span>
      </div>
    </section>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { formatBytes, formatSpeed, formatRemainingTime } from '../utils/format'
import type { Download } from '../types/download'
import { useI18n } from '../i18n'

const { t } = useI18n()

const props = defineProps<{
  downloads: Download[]
}>()

const activeCount = computed(() =>
  props.downloads.filter(d => d.status === 'active' || d.status === 'waiting').length
)

const completedCount = computed(() =>
  props.downloads.filter(d => d.status === 'complete').length
)

const failedCount = computed(() =>
  props.downloads.filter(d => d.status === 'error').length
)

const pausedCount = computed(() =>
  props.downloads.filter(d => d.status === 'paused').length
)

const totalSizeBytes = computed(() =>
  props.downloads.reduce((sum, d) => sum + (Number.isFinite(parseSize(d.size)) ? parseSize(d.size) : 0), 0)
)

const totalSizeText = computed(() => formatBytes(totalSizeBytes.value))

const todayCount = computed(() => {
  const start = new Date()
  start.setHours(0, 0, 0, 0)
  const ts = start.getTime()
  return props.downloads.filter(d => Number(d.added_at || 0) >= ts).length
})

const totalDownloadSpeed = computed(() => {
  const bytes = props.downloads
    .filter(d => d.status === 'active' || d.status === 'waiting')
    .reduce((sum, d) => sum + parseSpeed(d.speed), 0)
  return formatSpeed(bytes)
})

const totalUploadSpeed = computed(() => {
  return '0 B/s'
})

const activeDownloads = computed(() =>
  props.downloads
    .filter(d => d.status === 'active' || d.status === 'waiting')
    .sort((a, b) => (b.progress || 0) - (a.progress || 0))
)

const recentActivity = computed(() =>
  props.downloads
    .filter(d => d.status === 'complete' || d.status === 'error')
    .sort((a, b) => (b.added_at || 0) - (a.added_at || 0))
    .slice(0, 5)
)

const speedHistory = computed(() => {
  const samples = props.downloads
    .filter(d => d.status === 'active' && d.speedHistory?.length)
    .flatMap(d => d.speedHistory || [])
  if (samples.length === 0) return Array(16).fill(2)
  const last16 = samples.slice(-16)
  const max = Math.max(...last16, 1)
  return last16.map(s => Math.max(4, (s / max) * 100))
})

function parseSize(size: string | number | undefined): number {
  if (typeof size === 'number') return size
  if (!size) return 0
  const text = String(size).trim()
  const token = text.includes('/') ? text.split('/').pop()!.trim() : text
  const m = token.match(/([0-9.]+)\s*([KMGTP]?i?B)?/i)
  if (!m) return 0
  const v = Number(m[1])
  const u = (m[2] || '').toUpperCase()
  const mult: Record<string, number> = { '': 1, 'B': 1, 'KB': 1024, 'MB': 1048576, 'GB': 1073741824, 'TB': 1099511627776, 'KIB': 1024, 'MIB': 1048576, 'GIB': 1073741824, 'TIB': 1099511627776 }
  return v * (mult[u] || 1)
}

function parseSpeed(speed: string | undefined): number {
  if (!speed) return 0
  const m = speed.match(/([0-9.]+)\s*([KMGTP]?i?B)?\/s/i)
  if (!m) return 0
  const v = Number(m[1])
  const u = (m[2] || '').toUpperCase()
  const mult: Record<string, number> = { '': 1, 'B': 1, 'KB': 1024, 'MB': 1048576, 'GB': 1073741824 }
  return v * (mult[u] || 1)
}

function formatSize(size: string | undefined): string {
  if (!size) return '0 B'
  const token = String(size).includes('/') ? String(size).split('/').pop()!.trim() : String(size)
  const bytes = parseSize(token)
  return formatBytes(bytes)
}

function formatTime(ts: number | undefined): string {
  if (!ts) return ''
  const d = new Date(ts)
  const now = new Date()
  const diffMs = now.getTime() - d.getTime()
  const diffMin = Math.floor(diffMs / 60000)
  if (diffMin < 1) return t('Just now')
  if (diffMin < 60) return t('m ago').replace('{n}', String(diffMin))
  const diffHr = Math.floor(diffMin / 60)
  if (diffHr < 24) return t('h ago').replace('{n}', String(diffHr))
  return t('d ago').replace('{n}', String(Math.floor(diffHr / 24)))
}
</script>

<style scoped>
.home-page {
  color: var(--text);
  display: flex;
  flex-direction: column;
  gap: 14px;
  width: min(1100px, 100%);
  margin: 0 auto;
  padding: 4px 0 20px;
}

/* ── Stats Bar ── */
.stats-bar {
  display: grid;
  grid-template-columns: repeat(6, 1fr);
  gap: 10px;
}

.stat-chip {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 14px 12px;
  border-radius: 12px;
  background: var(--bg-card);
  border: 1px solid var(--border);
  transition: border-color 0.2s, transform 0.15s;
}

.stat-chip:hover {
  border-color: var(--border-strong);
  transform: translateY(-1px);
}

.chip-icon {
  width: 32px;
  height: 32px;
  border-radius: 8px;
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
}

.chip-icon .material-icons { font-size: 18px; }

.chip-blue   { background: rgba(var(--accent-rgb), 0.12); color: var(--accent-strong); }
.chip-green  { background: rgba(129, 199, 132, 0.12); color: var(--status-complete); }
.chip-red    { background: rgba(239, 83, 80, 0.12); color: var(--status-error); }
.chip-yellow { background: rgba(255, 213, 79, 0.12); color: #ffd54f; }
.chip-purple { background: rgba(179, 136, 255, 0.12); color: #b388ff; }
.chip-teal   { background: rgba(38, 198, 218, 0.12); color: #26c6da; }

.chip-info {
  display: flex;
  flex-direction: column;
}

.chip-val {
  font-size: 18px;
  font-weight: 800;
  line-height: 1.1;
  letter-spacing: -0.02em;
}

.chip-label {
  font-size: 11px;
  font-weight: 600;
  color: var(--text-dim);
  text-transform: uppercase;
  letter-spacing: 0.03em;
  margin-top: 1px;
}

/* ── Speed Card ── */
.speed-card {
  display: flex;
  align-items: center;
  gap: 24px;
  padding: 22px 28px;
  border-radius: 16px;
  background: var(--bg-card);
  border: 1px solid var(--border);
}

.speed-main {
  flex: 1;
}

.speed-row {
  display: flex;
  align-items: center;
  gap: 32px;
}

.speed-item {
  display: flex;
  align-items: center;
  gap: 10px;
}

.speed-arrow {
  font-size: 22px;
  color: var(--accent-strong);
}

.speed-value {
  font-size: 32px;
  font-weight: 800;
  letter-spacing: -0.03em;
}

.speed-label {
  font-size: 12px;
  font-weight: 600;
  color: var(--text-dim);
  text-transform: uppercase;
  letter-spacing: 0.04em;
}

.speed-divider {
  width: 1px;
  height: 40px;
  background: var(--border);
}

.speed-sub {
  margin: 6px 0 0;
  font-size: 13px;
  color: var(--text-dim);
}

.speed-visual {
  flex-shrink: 0;
}

.speed-bars {
  display: flex;
  align-items: flex-end;
  gap: 3px;
  height: 48px;
}

.speed-bar {
  width: 6px;
  min-height: 4px;
  border-radius: 3px;
  background: linear-gradient(to top, rgba(var(--accent-rgb), 0.3), var(--accent));
  transition: height 0.3s ease;
}

/* ── Panel ── */
.panel {
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: 14px;
  overflow: hidden;
}

.panel-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 16px 20px;
  border-bottom: 1px solid var(--border);
}

.panel-title {
  margin: 0;
  font-size: 14px;
  font-weight: 700;
  display: flex;
  align-items: center;
  gap: 8px;
}

.panel-title .material-icons {
  font-size: 18px;
  color: var(--accent-strong);
}

.panel-link {
  display: flex;
  align-items: center;
  gap: 4px;
  font-size: 12px;
  font-weight: 600;
  color: var(--accent-strong);
  text-decoration: none;
  transition: opacity 0.15s;
}

.panel-link .material-icons { font-size: 16px; }
.panel-link:hover { opacity: 0.8; }

/* ── Active Downloads ── */
.active-list {
  display: flex;
  flex-direction: column;
}

.active-item {
  display: flex;
  align-items: center;
  gap: 16px;
  padding: 12px 20px;
  border-bottom: 1px solid var(--border);
  transition: background 0.15s;
}

.active-item:last-child { border-bottom: none; }
.active-item:hover { background: var(--hover-bg); }

.active-info {
  flex: 1;
  min-width: 0;
}

.active-name {
  display: block;
  font-size: 13px;
  font-weight: 600;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.active-meta {
  font-size: 11px;
  color: var(--text-dim);
  margin-top: 2px;
}

.active-progress {
  display: flex;
  align-items: center;
  gap: 10px;
  flex-shrink: 0;
}

.progress-track {
  width: 120px;
  height: 6px;
  background: rgba(255, 255, 255, 0.06);
  border-radius: 3px;
  overflow: hidden;
}

.progress-fill {
  height: 100%;
  background: var(--accent);
  border-radius: 3px;
  transition: width 0.3s ease;
}

.progress-pct {
  font-size: 12px;
  font-weight: 700;
  color: var(--text-dim);
  width: 36px;
  text-align: right;
}

/* ── Activity ── */
.activity-list {
  display: flex;
  flex-direction: column;
}

.activity-item {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 11px 20px;
  border-bottom: 1px solid var(--border);
  transition: background 0.15s;
}

.activity-item:last-child { border-bottom: none; }
.activity-item:hover { background: var(--hover-bg); }

.activity-icon {
  width: 28px;
  height: 28px;
  border-radius: 50%;
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
}

.activity-icon .material-icons { font-size: 16px; }

.act-success { background: rgba(129, 199, 132, 0.12); color: var(--status-complete); }
.act-error   { background: rgba(239, 83, 80, 0.12); color: var(--status-error); }

.activity-info {
  flex: 1;
  min-width: 0;
}

.activity-name {
  display: block;
  font-size: 13px;
  font-weight: 600;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.activity-meta {
  font-size: 11px;
  color: var(--text-dim);
  margin-top: 1px;
}

.activity-time {
  font-size: 11px;
  color: var(--text-dim);
  flex-shrink: 0;
}

/* ── Empty State ── */
.empty-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  padding: 28px 20px;
  color: var(--text-dim);
}

.empty-state .material-icons {
  font-size: 32px;
  margin-bottom: 8px;
  opacity: 0.4;
}

.empty-state p {
  margin: 0;
  font-size: 13px;
  font-weight: 600;
  color: var(--text-muted);
}

.empty-state span {
  font-size: 12px;
  margin-top: 2px;
}

/* ── Responsive ── */
@media (max-width: 900px) {
  .stats-bar {
    grid-template-columns: repeat(3, 1fr);
  }
  .speed-row {
    flex-direction: column;
    align-items: flex-start;
    gap: 12px;
  }
  .speed-divider { display: none; }
}

@media (max-width: 600px) {
  .stats-bar {
    grid-template-columns: repeat(2, 1fr);
  }
  .speed-card {
    flex-direction: column;
    align-items: flex-start;
  }
  .speed-visual { display: none; }
  .active-progress { display: none; }
}
</style>
