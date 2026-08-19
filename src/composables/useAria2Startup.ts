import { invoke } from '@tauri-apps/api/core'

export function useAria2Startup() {
  async function startAria2(): Promise<boolean> {
    try {
      const settings = await invoke<Record<string, unknown>>('get_all_settings')
      await invoke('start_aria2_with_settings', { settings })

      const deadline = Date.now() + 3000
      while (Date.now() < deadline) {
        try {
          await invoke('get_aria2_version')
          return true
        } catch {
          await new Promise((r) => setTimeout(r, 100))
        }
      }
    } catch (error) {
      console.warn('Aria2 startup failed:', error)
    }
    return false
  }

  async function retryAria2(maxRetries = 3, intervalMs = 5000): Promise<void> {
    for (let i = 1; i <= maxRetries; i++) {
      try {
        console.log(`[↻] Retrying aria2 startup (attempt ${i})`)
        const settings = await invoke<Record<string, unknown>>('get_all_settings')
        await invoke('start_aria2_with_settings', { settings })

        const deadline = Date.now() + 3000
        while (Date.now() < deadline) {
          try {
            await invoke('get_aria2_version')
            console.log(`[✓] Aria2 started on retry ${i}`)
            return
          } catch {
            await new Promise((r) => setTimeout(r, 200))
          }
        }
      } catch (retryErr) {
        console.error(`[!] Retry ${i} failed:`, retryErr)
      }

      if (i < maxRetries) {
        await new Promise((r) => setTimeout(r, intervalMs))
      }
    }
    console.error(`[✗] Aria2 failed to start after ${maxRetries} retries`)
  }

  return { startAria2, retryAria2 }
}
