import { invoke } from '@tauri-apps/api/core'
import type { Download } from './types/download'

export async function upsertDownload(d: Partial<Download> & { gid: string; filename: string }) {
  try {
    const addedAt = d.added_at || Date.now()

    const record = {
      gid: d.gid,
      filename: d.filename,
      url: d.url || '',
      size: d.size ?? '0',
      status: d.status ?? 'unknown',
      progress: d.progress ?? 0,
      // Backend DownloadRecord has no serde rename_all, so it expects
      // snake_case field names. Sending `addedAt`/`ytdlpToken`/etc. made every
      // frontend upsert fail with `missing field added_at` — which silently
      // broke frontend-side persistence (e.g. marking an orphan as error).
      added_at: addedAt,
      path: d.path || null,
      connections: d.connections ?? 1,
      source: d.source || null,
      ytdlp_token: d.ytdlp_token || null,
      proxy: d.proxy || null,
    }

    await invoke('upsert_download', { record })
  } catch (error) {
    console.error('Failed to upsert download:', error)
  }
}

export async function removeDownloadFromDb(gid: string) {
  try {
    await invoke('remove_download_from_db', { gid })
  } catch (error) {
    console.error('Failed to remove download from db:', error)
  }
}

export async function getAllDownloadsFromDb(): Promise<Download[]> {
  try {
    return await invoke<Download[]>('get_all_downloads_from_db', {})
  } catch (error) {
    console.error('Failed to get all downloads from db:', error)
    return []
  }
}
