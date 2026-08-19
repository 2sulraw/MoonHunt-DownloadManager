/**
 * Download limit tracking: monitors cumulative bytes downloaded,
 * pauses all downloads when the limit is reached.
 */
import { ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import type { Aria2DownloadItem, Settings } from '../types/settings'

// Module-level singleton state
const downloadLimitEnabled = ref(false)
const downloadLimitBytes = ref(0)
const downloadLimitUsedBytes = ref(0)
const downloadLimitReached = ref(false)

// Per-GID last-seen completedLength for delta tracking
const prevCompletedMap = new Map<string, number>()
// Track which GIDs have reported a non-zero completedLength (verified real progress)
const verifiedGids = new Set<string>()
const seedCount = ref(0)

// Callback invoked when limit is reached (set by MainLayout)
const onLimitReached = ref<((usedBytes: number, limitBytes: number) => void) | null>(null)

// Periodic persist timer
let persistTimer: number | null = null

export function useDownloadLimit() {

  function seedFromLiveItems(items: Aria2DownloadItem[]) {
    for (const item of items) {
      const completed = parseInt(item.completedLength || '0', 10)
      const prev = prevCompletedMap.get(item.gid)
      if (prev === undefined || completed >= prev) {
        prevCompletedMap.set(item.gid, completed)
      }
      if (completed > 0) {
        verifiedGids.add(item.gid)
      }
    }
    seedCount.value++
  }

  function trackBytes(items: Aria2DownloadItem[]) {
    for (const item of items) {
      const completed = parseInt(item.completedLength || '0', 10)
      const prev = prevCompletedMap.get(item.gid) || 0
      const delta = Math.max(0, completed - prev)
      prevCompletedMap.set(item.gid, completed)
      if (!verifiedGids.has(item.gid)) {
        if (completed > 0) verifiedGids.add(item.gid)
        continue
      }
      if (downloadLimitEnabled.value && !downloadLimitReached.value && delta > 0) {
        downloadLimitUsedBytes.value += delta
      }
    }
  }

  /** Check if the limit has been reached. Returns true if so. */
  function checkLimit(): boolean {
    if (!downloadLimitEnabled.value) return false
    if (downloadLimitReached.value) return false
    if (downloadLimitBytes.value <= 0) return false

    if (downloadLimitUsedBytes.value >= downloadLimitBytes.value) {
      downloadLimitReached.value = true
      return true
    }
    return false
  }

  /** Pause all downloads and invoke the alert callback. */
  async function pauseAndNotify() {
    try {
      await invoke('pause_all_downloads')
    } catch (e) {
      console.error('[!] Failed to pause all downloads:', e)
    }
    if (onLimitReached.value) {
      onLimitReached.value(downloadLimitUsedBytes.value, downloadLimitBytes.value)
    }
  }

  /** Reset the counter. Downloads stay paused. */
  function resetLimit() {
    downloadLimitUsedBytes.value = 0
    downloadLimitReached.value = false
    prevCompletedMap.clear()
    verifiedGids.clear()
    seedCount.value = 0
    persistUsedBytes()
  }

  async function loadLimitSettings() {
    try {
      const settings = await invoke<Settings>('get_all_settings')
      downloadLimitEnabled.value = settings?.downloadLimitEnabled ?? false
      downloadLimitBytes.value = settings?.downloadLimitBytes ?? 0
      downloadLimitUsedBytes.value = settings?.downloadLimitUsedBytes ?? 0
      downloadLimitReached.value = false
    } catch (e) {
      console.error('[!] Failed to load limit settings:', e)
    }
  }

  async function saveLimitSettings(enabled: boolean, limitBytes: number) {
    try {
      const settings = await invoke<Settings>('get_all_settings')
      settings.downloadLimitEnabled = enabled
      settings.downloadLimitBytes = limitBytes
      settings.downloadLimitUsedBytes = downloadLimitUsedBytes.value
      await invoke('save_all_settings', { settings })

      downloadLimitEnabled.value = enabled
      downloadLimitBytes.value = limitBytes

      if (enabled && limitBytes > 0 && downloadLimitUsedBytes.value >= limitBytes && !downloadLimitReached.value) {
        downloadLimitReached.value = true
        await pauseAndNotify()
      }
    } catch (e) {
      console.error('[!] Failed to save limit settings:', e)
    }
  }

  async function persistUsedBytes() {
    try {
      const settings = await invoke<Settings>('get_all_settings')
      settings.downloadLimitUsedBytes = downloadLimitUsedBytes.value
      await invoke('save_all_settings', { settings })
    } catch (e) {
      console.error('[!] Failed to persist used bytes:', e)
    }
  }

  /** Start periodic persist (every 10s). */
  function startPeriodicPersist() {
    if (persistTimer) return
    persistTimer = window.setInterval(() => {
      if (downloadLimitEnabled.value && downloadLimitUsedBytes.value > 0) {
        persistUsedBytes()
      }
    }, 10000)
  }

  /** Stop periodic persist. */
  function stopPeriodicPersist() {
    if (persistTimer) {
      clearInterval(persistTimer)
      persistTimer = null
    }
  }

  return {
    downloadLimitEnabled,
    downloadLimitBytes,
    downloadLimitUsedBytes,
    downloadLimitReached,
    onLimitReached,
    seedCount,
    seedFromLiveItems,
    trackBytes,
    checkLimit,
    pauseAndNotify,
    resetLimit,
    loadLimitSettings,
    saveLimitSettings,
    persistUsedBytes,
    startPeriodicPersist,
    stopPeriodicPersist,
  }
}
