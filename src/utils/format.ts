/** Shared human-readable formatters for the UI. */

export function formatBytes(bytes: number): string {
  if (!bytes || bytes <= 0 || !isFinite(bytes)) return '0 B'
  const units = ['B', 'KB', 'MB', 'GB', 'TB']
  const i = Math.min(Math.floor(Math.log(bytes) / Math.log(1024)), units.length - 1)
  const value = bytes / Math.pow(1024, i)
  return i === 0 ? `${bytes} B` : `${value.toFixed(1)} ${units[i]}`
}

/** Alias used by download cards / layout. */
export function formatFileSize(bytes: number): string {
  return formatBytes(bytes)
}

/** Parse a single human-readable size like "34.0 MB" into bytes. */
export function parseFormattedSize(value: string | null | undefined): number {
  if (!value) return 0
  const m = value.trim().match(/([0-9.]+)\s*([KMGTP]?i?B)?/i)
  if (!m) return 0
  const v = Number(m[1])
  const u = (m[2] || '').toUpperCase()
  const mult: Record<string, number> = {
    B: 1,
    KB: 1024,
    MB: 1048576,
    GB: 1073741824,
    TB: 1099511627776,
  }
  return v * (mult[u] || 1)
}

/**
 * Parse a download size, preferring the max/total side when it is stored as
 * "completed / total". Completed rows may only store the final file size.
 */
export function parseDownloadSize(value: string | null | undefined): number {
  if (!value) return 0
  const parts = value.split('/').map((part) => parseFormattedSize(part))
  return Math.max(...parts)
}

export function formatSpeed(bytesPerSecond: number): string {
  if (!bytesPerSecond || bytesPerSecond <= 0 || !isFinite(bytesPerSecond)) return '0 B/s'
  return `${formatBytes(bytesPerSecond)}/s`
}

/** Estimate seconds remaining from raw byte totals and current speed. */
export function calculateRemainingSeconds(
  totalBytes: number,
  completedBytes: number,
  bytesPerSecond: number,
): number | undefined {
  if (
    !Number.isFinite(totalBytes) ||
    !Number.isFinite(completedBytes) ||
    !Number.isFinite(bytesPerSecond) ||
    totalBytes <= 0 ||
    bytesPerSecond <= 0
  ) {
    return undefined
  }
  const remaining = totalBytes - completedBytes
  if (remaining <= 0) return undefined
  return Math.ceil(remaining / bytesPerSecond)
}

/** Compact human-readable ETA, e.g. "12m 30s" or "1h 5m". */
export function formatRemainingTime(seconds: number | null | undefined): string {
  if (seconds == null || !Number.isFinite(seconds) || seconds <= 0) return '—'
  const total = Math.ceil(seconds)
  if (total < 60) return `${total}s`

  const minutes = Math.floor(total / 60)
  if (minutes < 60) {
    const secs = total % 60
    return secs ? `${minutes}m ${secs}s` : `${minutes}m`
  }

  const hours = Math.floor(minutes / 60)
  if (hours < 24) {
    const mins = minutes % 60
    return mins ? `${hours}h ${mins}m` : `${hours}h`
  }

  const days = Math.floor(hours / 24)
  const hrs = hours % 24
  return hrs ? `${days}d ${hrs}h` : `${days}d`
}

export function formatDateTime(value: unknown): string {
  if (value == null || value === '') return '—'
  const n = typeof value === 'number' ? value : Number(value)
  const d = Number.isFinite(n) ? new Date(n) : new Date(String(value))
  if (isNaN(d.getTime())) return String(value)
  return d.toLocaleString()
}
