export type DownloadCategoryMode = 'formatted' | 'packaged' | 'manual'

const EXACT_FORMAT_FOLDERS: Record<string, string> = {
  mp4: 'mp4',
  zip: 'zip',
  exe: 'exe',
}

const FORMAT_GROUPS: Array<{ folder: string; exts: string[] }> = [
  { folder: 'video', exts: ['mkv', 'webm', 'mov', 'avi', 'flv', 'wmv', 'm4v'] },
  { folder: 'audio', exts: ['mp3', 'flac', 'wav', 'm4a', 'ogg', 'aac', 'aiff', 'aif', 'wma'] },
  { folder: 'image', exts: ['jpg', 'jpeg', 'png', 'gif', 'webp', 'svg', 'bmp', 'ico'] },
  { folder: 'document', exts: ['pdf', 'doc', 'docx', 'txt', 'md', 'rtf', 'odt', 'xls', 'xlsx', 'ppt', 'pptx'] },
  { folder: 'code', exts: ['js', 'ts', 'py', 'rs', 'go', 'cpp', 'c', 'h', 'java', 'cs', 'php', 'rb'] },
  { folder: 'archive', exts: ['rar', '7z', 'tar', 'gz', 'bz2', 'xz'] },
  { folder: 'installer', exts: ['msi', 'dmg', 'pkg', 'apk', 'appx'] },
  { folder: 'disk-image', exts: ['iso', 'img', 'vhd', 'vdi'] },
]

const PACKAGED_GROUPS: Array<{ folder: string; exts: string[] }> = [
  { folder: 'video', exts: ['mp4', 'mkv', 'webm', 'mov', 'avi', 'flv', 'wmv', 'm4v'] },
  { folder: 'audio', exts: ['mp3', 'aiff', 'aif', 'flac', 'wav', 'm4a', 'ogg', 'aac', 'wma'] },
  { folder: 'image', exts: ['jpg', 'jpeg', 'png', 'gif', 'webp', 'svg', 'bmp', 'ico'] },
  { folder: 'document', exts: ['pdf', 'doc', 'docx', 'txt', 'md', 'rtf', 'odt', 'xls', 'xlsx', 'ppt', 'pptx'] },
  { folder: 'code', exts: ['js', 'ts', 'py', 'rs', 'go', 'cpp', 'c', 'h', 'java', 'cs', 'php', 'rb'] },
  { folder: 'archive', exts: ['zip', 'rar', '7z', 'tar', 'gz', 'bz2', 'xz'] },
  { folder: 'installer', exts: ['exe', 'msi', 'dmg', 'pkg', 'apk', 'appx'] },
  { folder: 'disk-image', exts: ['iso', 'img', 'vhd', 'vdi'] },
]

export function sanitizeCategoryFolder(name: string): string {
  const cleaned = name
    .trim()
    .replace(/[<>:"/\\|?*]/g, '_')
    .replace(/\s+/g, ' ')
    .replace(/^\.+$/, '_')
    .slice(0, 80)
  return cleaned || 'uncategorized'
}

export function extractFileExtension(value: string): string {
  const name = value.split(/[\\/]/).pop() || value
  const clean = name.split('?')[0]
  const match = clean.match(/\.([a-zA-Z0-9]{1,10})$/)
  return match ? match[1].toLowerCase() : ''
}

export function formattedCategoryFolder(filenameOrUrl: string): string {
  const ext = extractFileExtension(filenameOrUrl)
  if (!ext) return 'other'
  const exact = EXACT_FORMAT_FOLDERS[ext]
  if (exact) return exact
  const group = FORMAT_GROUPS.find((g) => g.exts.includes(ext))
  return group ? group.folder : 'other'
}

export function packagedCategoryFolder(filenameOrUrl: string): string {
  const ext = extractFileExtension(filenameOrUrl)
  if (!ext) return 'other'
  const group = PACKAGED_GROUPS.find((g) => g.exts.includes(ext))
  return group ? group.folder : 'other'
}

export function joinDownloadPath(base: string, child: string): string {
  const folder = sanitizeCategoryFolder(child)
  const trimmed = base.trim().replace(/[\\/]+$/, '')
  return trimmed ? `${trimmed}\\${folder}` : folder
}

export function resolveCategoryDir(input: {
  enabled: boolean
  mode: DownloadCategoryMode
  selected: string
  filename: string
  baseDir: string
}): string | null {
  const base = input.baseDir.trim()
  if (!input.enabled) return base || null
  if (!base) return null
  if (input.mode === 'manual') {
    if (!input.selected.trim()) return null
    return joinDownloadPath(base, input.selected)
  }
  if (input.mode === 'packaged') {
    return joinDownloadPath(base, packagedCategoryFolder(input.filename))
  }
  return joinDownloadPath(base, formattedCategoryFolder(input.filename))
}
