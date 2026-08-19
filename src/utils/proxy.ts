/** Per-download proxy form state (matches ProxyField.vue model value). */
export interface ProxyFormValue {
  enabled: boolean
  type: string
  host: string
  port: number
  user: string
  pass: string
}

export const emptyProxyForm = (): ProxyFormValue => ({
  enabled: false,
  type: 'http',
  host: '',
  port: 1080,
  user: '',
  pass: '',
})

/**
 * Render a per-download proxy as a URL string for aria2 / the DB.
 * Returns null when the proxy is disabled or missing a host — meaning "inherit global".
 * HTTP only; SOCKS5 is not supported.
 */
export function serializeProxy(proxy: ProxyFormValue | null | undefined): string | null {
  if (!proxy || !proxy.enabled) return null
  const host = (proxy.host || '').trim()
  if (!host) return null
  const port = Number(proxy.port) > 0 ? proxy.port : 1080
  const creds = proxy.user || proxy.pass ? `${proxy.user}:${proxy.pass}@` : ''
  return `http://${creds}${host}:${port}`
}

/** Parse a stored proxy URL string (from the DB / Download.proxy) back into form state. */
export function parseProxyString(value: string | null | undefined): ProxyFormValue {
  if (!value) return emptyProxyForm()
  let rest = value
  const schemeMatch = /^([a-zA-Z][a-zA-Z0-9+.-]*):\/\//.exec(value)
  if (schemeMatch) rest = value.slice(schemeMatch[0].length)
  let creds = ''
  const at = rest.lastIndexOf('@')
  if (at >= 0) {
    creds = rest.slice(0, at)
    rest = rest.slice(at + 1)
  }
  const [user = '', pass = ''] = creds.includes(':') ? creds.split(':') : [creds, '']
  const hostPort = rest
  let host = hostPort
  let port = 0
  const colon = hostPort.lastIndexOf(':')
  if (colon > 0 && /^\d+$/.test(hostPort.slice(colon + 1))) {
    host = hostPort.slice(0, colon)
    port = Number(hostPort.slice(colon + 1))
  }
  return {
    enabled: true,
    type: 'http',
    host,
    port: port || 1080,
    user,
    pass,
  }
}