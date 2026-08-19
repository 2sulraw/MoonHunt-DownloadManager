/**
 * Download list orchestration: poll aria2, merge DB history, normalize rows.
 */
import { ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import {
  upsertDownload,
  removeDownloadFromDb,
  getAllDownloadsFromDb,
} from '../db'
import type { Download } from '../types/download'
import type { Aria2DownloadItem } from '../types/settings'
import { normalizeStatus } from '../types/download'
import { formatFileSize, formatSpeed, calculateRemainingSeconds } from '../utils/format'
import { useDownloadLimit } from './useDownloadLimit'
import { useGroups } from './useGroups'

const MAX_SPEED_SAMPLES = 30

interface LiveStatsUpdate {
  gid: string
  connections: number
  remainingSeconds?: number | null
}

function normalizeUrlForMatch(u: string): string {
  return (u || '').toLowerCase().split(/[?#]/)[0]
}

function isDirectYtDlpGid(gid: string): boolean {
  return gid.startsWith('yt-dlp-direct-')
}

/**
 * Map a live aria2 RPC item into the shared Download model.
 * Prefer backend-persisted `source` / `ytdlp_token` when the row is later
 * merged with DB history. For live-only rows, only trust explicit header token.
 */
function mapAria2Item(
  item: Aria2DownloadItem,
  getRequestedConnectionsForUrl: (url: string) => number | undefined,
  speedHistoryMap: Map<string, number[]>,
): Download {
  const total = parseInt(item.totalLength || '0', 10)
  const completed = parseInt(item.completedLength || '0', 10)
  const progress = total > 0 ? Math.round((completed / total) * 100) : 0
  const sizeDisplay =
    total > 0
      ? `${formatFileSize(completed)} / ${formatFileSize(total)}`
      : '0 KB / 0 KB'

  let status = normalizeStatus(item.status)
  if ((item.status || '').toLowerCase() === 'waiting') status = 'waiting'

  let errorMessage = ''
  let errorCode = ''
  if (item.errorCode && status === 'error') {
    errorCode = item.errorCode
    errorMessage = item.errorMessage || `Error code: ${item.errorCode}`
  }

  let uri = ''
  if (item.files?.[0]?.uris?.[0]?.uri) {
    uri = item.files[0].uris[0].uri
  }

  const displayProgress = status === 'error' || status === 'complete' ? 100 : progress
  const requestedConnections = getRequestedConnectionsForUrl(uri)
  const liveConnections = parseInt(item.connections || item.split || '1', 10)
  const isLive = status === 'active' || status === 'waiting'

  // Prefer explicit token from backend handoff; avoid host heuristics.
  const extractedToken = extractYtDlpTokenSimple(item)
  let sourceTag: string | undefined = extractedToken ? 'yt-dlp' : undefined
  // Drop tag once the transfer is no longer live
  if (sourceTag && !['active', 'waiting'].includes((item.status || '').toLowerCase())) {
    sourceTag = undefined
  }

  const rawSpeed = parseInt(item.downloadSpeed || '0', 10)
  const arr = speedHistoryMap.get(item.gid) || []
  arr.push(rawSpeed)
  if (arr.length > MAX_SPEED_SAMPLES) arr.shift()
  speedHistoryMap.set(item.gid, arr)

  return {
    gid: item.gid,
    filename: item.files?.[0]?.path?.split(/[\\/]/).pop() || item.gid,
    path: item.files?.[0]?.path || '',
    progress: displayProgress,
    status,
    speed: formatSpeed(rawSpeed),
    size: sizeDisplay,
    total: total,
    url: uri,
    uri,
    connections: requestedConnections,
    activeConnections: isLive ? liveConnections : undefined,
    remainingSeconds: isLive
      ? calculateRemainingSeconds(total, completed, rawSpeed)
      : undefined,
    errorMessage,
    errorCode,
    source: sourceTag,
    ytdlp_token: extractedToken || undefined,
    speedHistory: arr,
  }
}

/** Look only for the X-Moonhunt-YTDLP header the backend sets. */
function extractYtDlpTokenSimple(item: Aria2DownloadItem): string | null {
  const headerName = 'x-moonhunt-ytdlp'
  const candidates: unknown[] = []
  if (item.requestHeaders) candidates.push(item.requestHeaders)
  if (item.files?.[0]?.requestHeaders) candidates.push(item.files[0].requestHeaders)
  if (item.files?.[0]?.uris?.[0]?.requestHeaders) {
    candidates.push(item.files[0].uris[0].requestHeaders)
  }

  for (const c of candidates) {
    if (!c) continue
    if (Array.isArray(c)) {
      for (const h of c) {
        if (!h) continue
        if (typeof h === 'string') {
          const idx = h.toLowerCase().indexOf(headerName + ':')
          if (idx !== -1) {
            const rest = h.substring(idx + headerName.length + 1).trim()
            const val = rest.split(/[;\r\n]/)[0].trim()
            if (val) return val
          }
        } else if (typeof h === 'object' && h !== null && 'name' in h && 'value' in h) {
          const hdr = h as { name: string; value: string }
          if (String(hdr.name).toLowerCase() === headerName) {
            return String(hdr.value)
          }
        }
      }
    } else if (typeof c === 'object' && c !== null) {
      for (const k of Object.keys(c as Record<string, unknown>)) {
        if (String(k).toLowerCase() === headerName) return String((c as Record<string, unknown>)[k])
      }
    }
  }
  return null
}

export function useDownloads() {
  const downloads = ref<Download[]>([])
  const speedHistoryMap: Map<string, number[]> = new Map()
  const firstSeenAtMap: Map<string, number> = new Map()
  const cancelledGids: Set<string> = new Set()
  const requestedConnectionsByUrl = new Map<string, number>()
  const { seedCount, seedFromLiveItems, trackBytes, checkLimit, pauseAndNotify } = useDownloadLimit()
  const { groups, updateGroup } = useGroups()

  let isRefreshing = false
  let pendingRefresh = false
  let refreshInterval: number | null = null
  let fileStateCheckInFlight = false
  let lastFileStateCheck = 0
  const FILE_STATE_CHECK_INTERVAL_MS = 30000
  const FILE_STATE_RECHECK_REMOVED_MS = 300000
  let cachedDbDownloads: Download[] | null = null
  const lastFileStateCheckByGid = new Map<string, number>()
  // On a fresh boot aria2 is a clean process that definitely does NOT hold any
  // GID from the previous session. So on the first poll, a DB row that says
  // active/paused/waiting but is absent from aria2 is an orphan by definition —
  // mark it error immediately instead of letting it sit as a stuck "active 0/0".
  // The grace window below only applies mid-session, where a download may
  // legitimately be in the stopped-history race.
  let isFirstPoll = true
  // Tracks how long an active/paused download has been absent from aria2's live
  // list. A brief absence is the stopped-history race; persistence past the
  // grace window means a crash/restart orphaned the transfer, so it must be
  // terminal (error) rather than stuck at 0/0 "active" forever.
  const missingSince = new Map<string, number>()
  const MISSING_ACTIVE_GRACE_MS = 15000
  // Backend events and periodic refreshes can arrive out of order. Serialize
  // every list mutation so a stale DB/aria2 snapshot can never overwrite a
  // fresher live progress event.
  let processingQueue: Promise<void> = Promise.resolve()

  function enqueueDownloadUpdate(items: Aria2DownloadItem[]): Promise<void> {
    const run = processingQueue.then(() => processAria2Items(items))
    processingQueue = run.catch(() => {})
    return run
  }

  function enqueueLiveStatsUpdate(updates: LiveStatsUpdate[]): Promise<void> {
    const run = processingQueue.then(() => applyLiveStatsUpdates(updates))
    processingQueue = run.catch(() => {})
    return run
  }

  function applyLiveStatsUpdates(updates: LiveStatsUpdate[]) {
    const byGid = new Map(updates.map((u) => [u.gid, u]))
    let changed = false
    const next = downloads.value.map((d) => {
      const update = byGid.get(d.gid)
      if (update !== undefined) {
        const remainingSeconds = update.remainingSeconds ?? undefined
        if (
          d.activeConnections === update.connections &&
          d.remainingSeconds === remainingSeconds
        ) {
          return d
        }
        changed = true
        return {
          ...d,
          activeConnections: update.connections,
          remainingSeconds,
        }
      }
      if (
        (d.status === 'active' || d.status === 'waiting') &&
        (d.activeConnections !== undefined || d.remainingSeconds !== undefined)
      ) {
        changed = true
        return { ...d, activeConnections: undefined, remainingSeconds: undefined }
      }
      return d
    })
    if (changed) downloads.value = next
  }

  function rememberRequestedConnections(url: string, connections: number) {
    if (!url) return
    requestedConnectionsByUrl.set(normalizeUrlForMatch(url), connections)
  }

  function getRequestedConnectionsForUrl(url: string): number | undefined {
    return requestedConnectionsByUrl.get(normalizeUrlForMatch(url))
  }

  function scheduleFileStateCheck() {
    if (fileStateCheckInFlight) return
    if (document.hidden) return
    const now = Date.now()
    if (now - lastFileStateCheck < FILE_STATE_CHECK_INTERVAL_MS) return
    lastFileStateCheck = now
    fileStateCheckInFlight = true

    const gids = downloads.value
      .filter(
        (d) =>
          d.status === 'complete' &&
          now - (lastFileStateCheckByGid.get(d.gid) || 0) >=
            (d.fileState === 'removed' ? FILE_STATE_RECHECK_REMOVED_MS : FILE_STATE_CHECK_INTERVAL_MS),
      )
      .map((d) => d.gid)
    if (!gids.length) {
      fileStateCheckInFlight = false
      return
    }

    invoke<Array<{ gid: string; fileState?: Download['fileState']; path?: string }>>(
      'check_download_file_states',
      { gids },
    )
      .then((rows) => {
        const byGid = new Map(rows.map((r) => [r.gid, r]))
        for (const r of rows) lastFileStateCheckByGid.set(r.gid, now)
        for (const gid of [...lastFileStateCheckByGid.keys()]) {
          if (!downloads.value.some((d) => d.gid === gid)) {
            lastFileStateCheckByGid.delete(gid)
          }
        }
        downloads.value = downloads.value.map((d) => {
          const info = byGid.get(d.gid)
          return info
            ? { ...d, fileState: info.fileState ?? d.fileState, path: info.path ?? d.path }
            : d
        })
      })
      .catch((error) => console.error('Failed to check download file states:', error))
      .finally(() => {
        fileStateCheckInFlight = false
      })
  }

  async function loadCompletedDownloads() {
    // Always read fresh from the DB. The backend poller owns terminal-state
    // transitions (e.g. orphaning a mid-download kill to `error`), and that can
    // happen between poll events. A 5s cache would keep re-serving the stale
    // `active` row and mask the backend's correction — showing a dead download
    // as a stuck 0/0 "active" forever.
    cachedDbDownloads = await getAllDownloadsFromDb()
    return cachedDbDownloads
  }

  /** Kept for Media page event compatibility; source is now backend-tagged. */
  function handleYtDlpStart(_e: { payload?: { url?: string }; url?: string }) {
    // yt-dlp downloads are persisted on the backend, so refresh the list
    // immediately when a media transfer starts.
    cachedDbDownloads = null
    refreshDownloads()
  }

  async function processAria2Items(items: Aria2DownloadItem[]) {
    console.log(`[poll] receive ${items.length} live aria2 items:`, items.map((i) => `${i.gid}:${i.status}:${i.completedLength}/${i.totalLength}`))
    if (seedCount.value < 4) {
      seedFromLiveItems(items)
    } else {
      trackBytes(items)
      if (checkLimit()) {
        pauseAndNotify()
      }
    }

    // Per-group download byte limits
    for (const group of groups.value) {
      if (!group.downloadLimitEnabled || group.downloadLimitBytes <= 0) continue

      const memberSet = new Set(group.memberGids)
      let groupTotalBytes = 0

      for (const item of items) {
        if (!memberSet.has(item.gid)) continue
        groupTotalBytes += parseInt(item.completedLength || '0', 10)
      }

      if (group.downloadLimitUsedBytes < group.downloadLimitBytes) {
        const newUsed = Math.max(group.downloadLimitUsedBytes, groupTotalBytes)
        if (newUsed !== group.downloadLimitUsedBytes) {
          updateGroup(group.id, { downloadLimitUsedBytes: newUsed })
        }

        if (newUsed >= group.downloadLimitBytes) {
          for (const item of items) {
            if (!memberSet.has(item.gid)) continue
            const st = (item.status || '').toLowerCase()
            if (st === 'active' || st === 'waiting') {
              invoke('pause_download', { gid: item.gid }).catch((e: unknown) =>
                console.error(`[group-limit] Failed to pause ${item.gid}:`, e),
              )
            }
          }
        }
      }
    }

    const live = items.map((item) => {
      const row = mapAria2Item(item, getRequestedConnectionsForUrl, speedHistoryMap)
      if (!row.added_at) {
        if (!firstSeenAtMap.has(row.gid)) firstSeenAtMap.set(row.gid, Date.now())
        row.added_at = firstSeenAtMap.get(row.gid)
      }
      return row
    })

    // Terminal persistence is owned by the backend poller (see lib.rs
    // persist_terminal_row). Drop the frontend writer/dedup entirely — it
    // duplicated the DB and raced the survival of `persistedCompletedGids`
    // across restarts, which was the root cause of completed downloads
    // reverting to "active" after a relaunch.

    const allCurrentGids = new Set([
      ...live.map((d) => d.gid),
      ...downloads.value.map((d) => d.gid),
    ])
    for (const gid of cancelledGids) {
      if (!allCurrentGids.has(gid)) cancelledGids.delete(gid)
    }
    for (const gid of speedHistoryMap.keys()) {
      if (!allCurrentGids.has(gid)) speedHistoryMap.delete(gid)
    }
    for (const gid of firstSeenAtMap.keys()) {
      if (!allCurrentGids.has(gid)) firstSeenAtMap.delete(gid)
    }

    const completedDownloads = await loadCompletedDownloads()
    const dbByGid = new Map(completedDownloads.map((c) => [c.gid, c]))

    const missingActive: Download[] = []
    const now = Date.now()
    const firstPoll = isFirstPoll
    isFirstPoll = false

    // On a fresh boot, aria2 is a clean process and cannot hold any GID from the
    // previous session. Sweep the DB rows themselves: any non-terminal row not in
    // the live set was mid-download when the app was killed. Display it as PAUSED
    // (so it never appears as a stuck "active 0/0") — but DO NOT write it back to
    // the DB. The backend poller owns orphan recovery: it re-adds the resumable
    // partial as a single paused aria2 task (replacing the old row) or flips it
    // to error if nothing is resumable. Writing here would duplicate rows
    // (original gid + backend's new gid) and break Resume.
    if (firstPoll) {
      const liveGidsFirst = new Set(live.map((d) => d.gid))
      for (const dbRow of completedDownloads) {
        if (isDirectYtDlpGid(dbRow.gid)) continue
        if (cancelledGids.has(dbRow.gid)) continue
        if (liveGidsFirst.has(dbRow.gid)) continue
        const st = normalizeStatus(dbRow.status)
        if (st !== 'active' && st !== 'paused' && st !== 'waiting') continue
        const orphaned: Download = { ...dbRow, status: 'paused' as Download['status'] }
        missingActive.push(orphaned)
      }
    }

    for (const prev of downloads.value) {
      if (
        (prev.status === 'active' || prev.status === 'paused') &&
        !live.find((l) => l.gid === prev.gid) &&
        !cancelledGids.has(prev.gid)
      ) {
        // Direct yt-dlp transfers are not visible to aria2. Their live state
        // arrives via downloads-changed events, so a fallback refresh must not
        // treat them as missing and re-persist an older progress snapshot.
        if (isDirectYtDlpGid(prev.gid)) continue
        const dbRow = dbByGid.get(prev.gid)
        if (dbRow && ['complete', 'error', 'cancelled'].includes(normalizeStatus(dbRow.status))) {
          continue
        }

        const seenMissingSince = missingSince.get(prev.gid)
        if (firstPoll) {
          // Fresh boot: aria2 is a clean process that never held this GID.
          // There is no stopped-history race to wait out — it's an orphan.
          // Show it paused; the backend re-adds the resumable partial or marks
          // error if nothing is resumable. Display-only: no DB write (backend
          // owns recovery, to avoid duplicate rows).
          const orphaned: Download = { ...prev, status: 'paused' as Download['status'] }
          missingActive.push(orphaned)
        } else if (seenMissingSince === undefined) {
          // First poll where aria2 lost the GID. Mark it and keep it visible
          // briefly — aria2 can move a finished download into stopped history
          // between snapshots.
          missingSince.set(prev.gid, now)
          missingActive.push(prev)
        } else if (now - seenMissingSince < MISSING_ACTIVE_GRACE_MS) {
          // Still within the stopped-history race window.
          missingActive.push(prev)
        } else {
          // Persistently missing past the grace window: the app was killed and
          // restarted mid-transfer, orphaning this download in aria2. Show it
          // paused; the backend re-adds the resumable partial or marks error.
          // Display-only: no DB write (backend owns recovery).
          missingSince.delete(prev.gid)
          const orphaned: Download = { ...prev, status: 'paused' as Download['status'] }
          missingActive.push(orphaned)
        }
      } else if ((prev.status === 'active' || prev.status === 'paused') && (live.find((l) => l.gid === prev.gid) || cancelledGids.has(prev.gid))) {
        // It came back or was explicitly cancelled — clear the tracking.
        missingSince.delete(prev.gid)
      }
    }

    const liveGids = new Set(live.map((d) => d.gid))
    const missingActiveGids = new Set(missingActive.map((d) => d.gid))
    const previousByGid = new Map(downloads.value.map((d) => [d.gid, d]))
    const terminalDirectGids = new Set(
      completedDownloads
        .filter(
          (c) =>
            isDirectYtDlpGid(c.gid) &&
            ['complete', 'error', 'cancelled'].includes(normalizeStatus(c.status)),
        )
        .map((c) => c.gid),
    )
    const directActive = downloads.value.filter(
      (d) =>
        isDirectYtDlpGid(d.gid) &&
        d.status === 'active' &&
        !terminalDirectGids.has(d.gid),
    )
    const directActiveGids = new Set(directActive.map((d) => d.gid))
    const completedNotInLive = completedDownloads
      .map((c) => {
        const prev = previousByGid.get(c.gid)
        return {
          ...c,
          status: normalizeStatus(c.status) as Download['status'],
          fileState: c.fileState ?? prev?.fileState,
          path: prev?.fileState ? prev.path || c.path : c.path,
        }
      })
      .filter(
        (c) =>
          !liveGids.has(c.gid) &&
          !directActiveGids.has(c.gid) &&
          !missingActiveGids.has(c.gid),
      )

    const liveMerged = live.map((row) => {
      const db = dbByGid.get(row.gid)
      const prev = previousByGid.get(row.gid)
      // aria2 reports totalLength/completedLength of 0 for a PAUSED download until
      // it actually resumes. The DB row (live-synced before a kill) holds the real
      // partial/full sizes — if the live item has no size info, prefer the DB's.
      const hasRealSize = row.total != null && row.total > 0
      const mergedSize = hasRealSize ? row.size : db?.size || row.size
      const mergedProgress = hasRealSize ? row.progress : db?.progress ?? row.progress
      return {
        ...row,
        size: mergedSize,
        progress: mergedProgress,
        connections: row.connections ?? db?.connections ?? prev?.connections,
        added_at: db?.added_at ?? prev?.added_at ?? row.added_at,
        fileState: prev?.fileState ?? row.fileState,
        path: prev?.fileState ? prev.path || row.path : row.path,
        proxy: row.proxy ?? db?.proxy ?? prev?.proxy,
        ...(db?.source && !row.source ? { source: db.source, ytdlp_token: db.ytdlp_token ?? row.ytdlp_token } : {}),
      }
    })

    downloads.value = [
      ...completedNotInLive,
      ...liveMerged,
      ...missingActive,
      ...directActive.filter((d) => !liveGids.has(d.gid)),
    ]
    console.log(
      `[poll] final UI rows: ${downloads.value.map((d) => `${d.gid}:${d.status}:${d.size}`).join(' | ')}`,
    )
    const activeRows = downloads.value.filter((d) => d.status === 'active' || d.status === 'waiting')
    if (activeRows.length) {
      console.log(`[poll] ACTIVE rows in UI (not from aria2 live?): ${activeRows.map((d) => `${d.gid}:${d.filename}:${d.size}`).join(' | ')}`)
    }
    scheduleFileStateCheck()
  }

  async function refreshDownloads() {
    if (isRefreshing) {
      pendingRefresh = true
      return
    }
    isRefreshing = true
    try {
      const items = (await invoke('get_all_downloads')) as Aria2DownloadItem[]
      await enqueueDownloadUpdate(items)
    } catch (error) {
      console.error(
        'Failed to fetch live downloads from aria2 (falling back to DB):',
        error,
      )
      try {
        const completedDownloads = await getAllDownloadsFromDb()
        const previousByGid = new Map(downloads.value.map((d) => [d.gid, d]))
        const previousDirectActive = new Map(
          downloads.value
            .filter((d) => isDirectYtDlpGid(d.gid) && d.status === 'active')
            .map((d) => [d.gid, d]),
        )
        const orphanedStatuses = new Set(['active', 'paused', 'waiting'])
        downloads.value = completedDownloads.map((c) => {
          const prev = previousByGid.get(c.gid)
          const liveDirect = previousDirectActive.get(c.gid)
          if (
            liveDirect &&
            !['complete', 'error', 'cancelled'].includes(normalizeStatus(c.status))
          ) {
            return liveDirect
          }
          const rawStatus = normalizeStatus(c.status)
          // aria2 just failed and we're falling back to the DB. Any non-terminal
          // row was mid-flight when the app was killed/restarted — aria2 has no
          // record of it yet, so it would show as a stuck 0/0 "active" ghost.
          // Show it paused so the backend poller's orphan recovery can re-add the
          // resumable partial under the same gid. Display-only: writing `error`
          // here raced the backend and silently killed resumable downloads.
          if (!isDirectYtDlpGid(c.gid) && orphanedStatuses.has(rawStatus)) {
            return { ...c, status: 'paused' as Download['status'] }
          }
          return {
            ...c,
            status: rawStatus as Download['status'],
            fileState: c.fileState ?? prev?.fileState,
            path: prev?.fileState ? prev.path || c.path : c.path,
          }
        })
        const snapshotGids = new Set(completedDownloads.map((c) => c.gid))
        const directActiveNotInDb = [...previousDirectActive.values()].filter(
          (d) => !snapshotGids.has(d.gid),
        )
        downloads.value = [...downloads.value, ...directActiveNotInDb]
        scheduleFileStateCheck()
      } catch (dbError) {
        console.error('Failed to load downloads from DB:', dbError)
        downloads.value = []
      }
    } finally {
      isRefreshing = false
      if (pendingRefresh) {
        pendingRefresh = false
        setTimeout(() => refreshDownloads(), 50)
      }
    }
  }

  async function cancelDownload(gid: string) {
    // 1. Remove from aria2 (stops transfer, purges from aria2 lists)
    //    This internally calls mark_cancelled so the poll filters it out
    await invoke('remove_download', { gid }).catch(() => {})
    // 2. Save to DB as cancelled (persists across restarts)
    const existing = downloads.value.find(d => d.gid === gid)
    if (existing) {
      await upsertDownload({ ...existing, status: 'cancelled' as Download['status'] })
      cachedDbDownloads = null
    }
    // 3. Track in frontend set to prevent re-persistence from poll
    cancelledGids.add(gid)
    // 4. Immediately update UI
    downloads.value = downloads.value.map(d =>
      d.gid === gid ? { ...d, status: 'cancelled' as Download['status'] } : d
    )
  }

  async function removeDownload(gid: string) {
    cancelledGids.add(gid)
    if (isDirectYtDlpGid(gid)) {
      // Direct yt-dlp downloads aren't aria2 tasks — cancel the child process.
      await invoke('cancel_yt_download', { gid })
    } else {
      await invoke('remove_download', { gid })
    }
    await removeDownloadFromDb(gid)
    cachedDbDownloads = null
    downloads.value = downloads.value.filter((d) => d.gid !== gid)
    speedHistoryMap.delete(gid)
    firstSeenAtMap.delete(gid)
    cancelledGids.delete(gid)
    await refreshDownloads()
  }

  function temporaryFastRefresh() {
    refreshDownloads()
  }

  function startPolling() {
    const unlistenFns: (() => void)[] = []

    // Primary: listen for backend-pushed events (no IPC overhead)
    listen<Aria2DownloadItem[]>('downloads-changed', (event) => {
      enqueueDownloadUpdate(event.payload).catch((error) =>
        console.error('Failed to process download update:', error),
      )
    }).then((unlisten) => {
      unlistenFns.push(unlisten)
    })

    // Live stats are pushed from the backend poller separately so the UI does
    // not need extra RPC or frontend polling.
    listen<LiveStatsUpdate[]>('live-stats-changed', (event) => {
      enqueueLiveStatsUpdate(event.payload).catch((error) =>
        console.error('Failed to apply live stats update:', error),
      )
    }).then((unlisten) => {
      unlistenFns.push(unlisten)
    })

    // yt-dlp direct downloads don't go through aria2, so refresh from the DB
    // as soon as the backend reports a terminal state.
    listen<{ gid: string; filename: string }>('yt-dl-complete', () => {
      cachedDbDownloads = null
      refreshDownloads()
    }).then((unlisten) => {
      unlistenFns.push(unlisten)
    })

    listen<{ gid: string; error: string }>('yt-dl-error', () => {
      cachedDbDownloads = null
      refreshDownloads()
    }).then((unlisten) => {
      unlistenFns.push(unlisten)
    })

    // Fallback: poll via IPC if backend events aren't available
    const startRefresh = (intervalMs: number) => {
      if (refreshInterval) clearInterval(refreshInterval)
      refreshInterval = window.setInterval(refreshDownloads, intervalMs)
    }
    startRefresh(10000) // Reduced from 1s to 10s — events handle real-time updates

    const onVisibilityChange = () => {
      if (document.hidden) {
        startRefresh(30000)
      } else {
        startRefresh(10000)
        refreshDownloads()
      }
    }
    document.addEventListener('visibilitychange', onVisibilityChange)
    return () => {
      if (refreshInterval) clearInterval(refreshInterval)
      unlistenFns.forEach((fn) => fn())
      document.removeEventListener('visibilitychange', onVisibilityChange)
    }
  }

  return {
    downloads,
    refreshDownloads,
    cancelDownload,
    removeDownload,
    temporaryFastRefresh,
    startPolling,
    rememberRequestedConnections,
    handleYtDlpStart,
  }
}
