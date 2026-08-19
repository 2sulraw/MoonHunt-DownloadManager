/** App settings persisted in SQLite. */
export interface Settings {
  downloadLocation: string
  maxConcurrent: number
  maxDownloadSpeed: number
  globalSpeedLimitEnabled?: boolean
  jsRuntime?: string
  proxyType: 'none' | 'http'
  proxyHost: string
  proxyPort: number
  proxyUser: string
  proxyPass: string
  rpcPort: number
  extensionPort: number
  extensionAuthEnabled?: boolean
  extensionAuthToken?: string
  openInBrowserProtocolEnabled?: boolean
  theme: string
  themeMode: string
  language?: string
  downloadLimitEnabled: boolean
  downloadLimitBytes: number
  downloadLimitUsedBytes: number
  downloadLimitUnit?: 'KB' | 'MB' | 'GB'
  verifySsl: boolean
  deleteFileOnCancel: boolean
  disableHardwareAcceleration: boolean
  debugLogging?: boolean
  torrentDownloadLocation?: string
  torrentMaxConnections?: number
  torrentDhtEnabled?: boolean
  torrentAutoStart?: boolean
  torrentUploadLimitEnabled?: boolean
  torrentUploadLimit?: number
}

/** Raw aria2 RPC item returned by get_all_downloads. */
export interface Aria2DownloadItem {
  gid: string
  status: string
  totalLength: string
  completedLength: string
  downloadSpeed: string
  uploadSpeed: string
  files: Aria2File[]
  connections?: string
  split?: string
  errorCode?: string
  errorMessage?: string
  requestHeaders?: unknown
}

export interface Aria2File {
  path: string
  length: string
  completedLength: string
  uris: { uri: string; status: string; downloadSpeed: string; requestHeaders?: unknown }[]
  requestHeaders?: unknown
}

/** Detailed download info from get_download_details. */
export interface DownloadDetails {
  status: string
  totalLength: string
  completedLength: string
  files: Aria2File[]
  source?: string[]
  startTime?: string
  completionTime?: string
  errorMessage?: string
}

/** Scheduled download action. */
export interface Schedule {
  id?: number
  downloadGid?: string
  url: string
  filename: string
  action: string
  scheduledTime: number
  repeatInterval?: number
  enabled: boolean
  createdAt: number
}

export type SortOption = 'added' | 'name' | 'size' | 'manual'

export type GroupOnComplete = 'none' | 'close' | 'sleep' | 'shutdown' | 'restart'

export interface GroupMember {
  gid: string
  filename: string
  status: string
  isPending: boolean
}
