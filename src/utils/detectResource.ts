/**
 * Download resource detection.
 *
 * Rules:
 * 1. Content must be ≤ 2048 chars (long payloads aren't URLs).
 * 2. Split into lines, filter empty lines.
 * 3. Max 200 lines.
 * 4. Each line must start with a recognized protocol tag.
 * 5. Single-line: just check the tag.
 * 6. Multi-line: every line must be a valid download URI.
 *
 * This rejects URLs embedded in prose, code, JSON, HTML, or log lines.
 */

const MAX_CHARS = 2048
const MAX_LINES = 200

/** All recognized protocol prefixes. */
export const ALL_PROTOCOLS = ['http://', 'https://', 'ftp://', 'magnet:', 'ed2k://', 'thunder://'] as const

export type ProtocolType = 'http' | 'ftp' | 'magnet' | 'ed2k' | 'thunder'

export interface ClipboardFilter {
  enable: boolean
  http: boolean
  ftp: boolean
  magnet: boolean
  ed2k: boolean
  thunder: boolean
}

const DEFAULT_FILTER: ClipboardFilter = {
  enable: true,
  http: true,
  ftp: true,
  magnet: true,
  ed2k: false,
  thunder: true,
}

function buildAllowedTags(filter?: ClipboardFilter): string[] {
  const f = filter || DEFAULT_FILTER
  if (!f.enable) return []
  const tags: string[] = []
  if (f.http) tags.push('http://', 'https://')
  if (f.ftp) tags.push('ftp://')
  if (f.magnet) tags.push('magnet:')
  if (f.ed2k) tags.push('ed2k://')
  if (f.thunder) tags.push('thunder://')
  return tags
}

function lineMatchesAllowed(line: string, allowedTags: string[]): boolean {
  const lower = line.toLowerCase()
  return allowedTags.some(tag => lower.startsWith(tag) && line.length > tag.length)
}

/**
 * Check if clipboard text is downloadable resource(s).
 *
 * @param content - Clipboard text
 * @param filter - Optional protocol filter
 * @returns true if content looks like one or more download URLs
 */
export function detectResource(content: string, filter?: ClipboardFilter): boolean {
  if (filter && !filter.enable) return false
  if (!content || content.length > MAX_CHARS) return false

  const lines = content
    .split(/\r?\n/)
    .map(l => l.trim())
    .filter(Boolean)

  if (lines.length === 0 || lines.length > MAX_LINES) return false

  const allowedTags = buildAllowedTags(filter)
  if (allowedTags.length === 0) return false

  // Single line: simple check
  if (lines.length === 1) {
    return lineMatchesAllowed(lines[0], allowedTags)
  }

  // Multi-line: every line must be a valid URI
  return lines.every(line => lineMatchesAllowed(line, allowedTags))
}

/**
 * Extract the first valid download URL from clipboard content.
 * Returns null if no valid URL found.
 */
export function extractFirstUrl(content: string, filter?: ClipboardFilter): string | null {
  if (filter && !filter.enable) return null
  if (!content || content.length > MAX_CHARS) return null

  const lines = content
    .split(/\r?\n/)
    .map(l => l.trim())
    .filter(Boolean)

  if (lines.length === 0 || lines.length > MAX_LINES) return null

  const allowedTags = buildAllowedTags(filter)
  if (allowedTags.length === 0) return null

  for (const line of lines) {
    if (lineMatchesAllowed(line, allowedTags)) return line
  }
  return null
}
