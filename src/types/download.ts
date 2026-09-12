/** Canonical download model shared across UI, DB, and actions. */

export type DownloadStatus =
  | 'active'
  | 'waiting'
  | 'paused'
  | 'complete'
  | 'error'
  | 'cancelled'
  | 'unknown'

export type DownloadSource = 'aria2' | 'yt-dlp' | string

export type DownloadFileState = 'present' | 'moved' | 'removed' | 'unknown'

export interface Download {
  gid: string
  filename: string
  url: string
  path?: string
  status: DownloadStatus
  progress: number
  size: string
  /** Raw total bytes reported by aria2; 0 when unknown (e.g. paused). */
  total?: number
  speed?: string
  /** Requested/configured number of connections used by aria2. */
  connections?: number
  /** Live connection count reported by aria2. */
  activeConnections?: number
  /** Estimated seconds remaining while active; undefined when unknown. */
  remainingSeconds?: number
  source?: DownloadSource
  ytdlp_token?: string
  added_at?: number
  errorMessage?: string
  errorCode?: string
  /** Rolling raw B/s samples for sparkline charts */
  speedHistory?: number[]
  uri?: string
  /** Whether the downloaded file still exists at its recorded location. */
  fileState?: DownloadFileState
  /** Per-download proxy override (e.g. `http://user:pass@host:port`); empty = inherit global. */
  proxy?: string
}

/** Normalize aria2 / DB status strings into a small UI enum. */
export function normalizeStatus(raw: string | undefined | null): DownloadStatus {
  const s = (raw || 'unknown').toLowerCase()
  if (s === 'removed') return 'cancelled'
  if (s === 'stopped') return 'complete'
  if (s === 'active' || s === 'waiting' || s === 'paused' || s === 'complete' || s === 'error' || s === 'cancelled') {
    return s
  }
  return 'unknown'
}

/** A single event in a download's activity log. */
export interface DownloadEvent {
  id: number
  gid: string
  timestamp: number
  eventType: 'state_change' | 'error' | 'speed_sample' | 'retry' | 'info'
  detail: string | null
}
