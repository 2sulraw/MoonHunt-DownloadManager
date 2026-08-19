/** Expand URL templates with range and alternative patterns. */

/** Expand [start-end] and {a,b,c} patterns in a URL template. */
export function expandTemplate(template: string): string[] {
  const tokens = template.split(/(\[[^\]]+\]|\{[^}]+\})/)
  if (tokens.length === 1) return template.trim() ? [template.trim()] : []

  const segments: string[][] = []
  for (const token of tokens) {
    const rangeMatch = token.match(/^\[(.+)\]$/)
    if (rangeMatch) {
      segments.push(expandRange(rangeMatch[1]))
      continue
    }
    const listMatch = token.match(/^\{(.+)\}$/)
    if (listMatch) {
      segments.push(listMatch[1].split(',').map(v => v.trim()).filter(Boolean))
      continue
    }
    segments.push(token ? [token] : [])
  }

  return cartesianProduct(segments)
}

function expandRange(spec: string): string[] {
  const parts = spec.split('-')
  if (parts.length !== 2) return [`[${spec}]`]
  const start = parseInt(parts[0], 10)
  const end = parseInt(parts[1], 10)
  if (isNaN(start) || isNaN(end)) return [`[${spec}]`]
  const pad = parts[0].length
  const result: string[] = []
  for (let i = start; i <= end; i++) {
    result.push(String(i).padStart(pad, '0'))
  }
  return result
}

function cartesianProduct(segments: string[][]): string[] {
  if (segments.length === 0) return ['']
  const [first, ...rest] = segments
  const restProduct = cartesianProduct(rest)
  const result: string[] = []
  for (const f of first) {
    for (const r of restProduct) {
      result.push(f + r)
    }
  }
  return result
}
