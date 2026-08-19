import { invoke } from '@tauri-apps/api/core'

/**
 * Mirrors the frontend's console output to the backend log file when debug
 * logging is enabled. The backend appends only when its own toggle is on, so
 * this is safe to call unconditionally — a fire-and-forget no-op otherwise.
 */

let enabled = false

export function setDebugLogBridge(on: boolean) {
  enabled = on
}

export function debugLogBridgeActive() {
  return enabled
}

function forward(method: 'log' | 'warn' | 'error', args: unknown[]) {
  if (!enabled) return
  const parts = args.map((a) => {
    try {
      return typeof a === 'string' ? a : JSON.stringify(a)
    } catch {
      return String(a)
    }
  })
  const line = `[frontend:${method}] ${parts.join(' ')}`
  // fire-and-forget; failures are irrelevant to the UI
  void invoke('append_log_line', { msg: line }).catch(() => {})
}

/** Install the console shim once. Returns an uninstall function. */
export function installConsoleBridge(): () => void {
  const origLog = console.log
  const origWarn = console.warn
  const origError = console.error
  console.log = (...args: unknown[]) => { forward('log', args); origLog(...args) }
  console.warn = (...args: unknown[]) => { forward('warn', args); origWarn(...args) }
  console.error = (...args: unknown[]) => { forward('error', args); origError(...args) }
  return () => {
    console.log = origLog
    console.warn = origWarn
    console.error = origError
  }
}
