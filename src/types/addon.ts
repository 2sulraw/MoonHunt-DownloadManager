export interface AddonManifest {
  id: string
  name: string
  version: string
  author: string
  entry: string
  icon: string
  description: string
  url_patterns: string[]
  permissions: string[]
  min_app_version: string
}

export interface AddonSource {
  type: string
  detail: string
}

export interface InstalledAddon {
  manifest: AddonManifest
  enabled: boolean
  source: AddonSource
  hash: string
  installed_at: number
}

export interface YtSearchResult {
  id: string
  title: string
  duration?: number | null
  uploader?: string | null
  webpage_url: string
}
