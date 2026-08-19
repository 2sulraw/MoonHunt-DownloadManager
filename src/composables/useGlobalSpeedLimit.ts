import { ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import type { Settings } from '../types/settings'
import { useGroups } from './useGroups'

const globalSpeedLimitEnabled = ref(false)
const globalSpeedLimitKb = ref(0)
const isApplying = ref(false)
const loaded = ref(false)
const { reapplyAllLimits } = useGroups()

async function loadGlobalSpeedLimit() {
  try {
    const settings = await invoke<Settings>('get_all_settings')
    globalSpeedLimitKb.value = settings.maxDownloadSpeed || 0
    globalSpeedLimitEnabled.value =
      settings.globalSpeedLimitEnabled ?? globalSpeedLimitKb.value > 0
    loaded.value = true
  } catch (error) {
    console.error('Failed to load global speed limit:', error)
  }
}

async function setGlobalSpeedLimit(enabled: boolean, speedKb: number) {
  if (isApplying.value) return
  isApplying.value = true
  try {
    const settings = await invoke<Settings>('get_all_settings')
    const limitKb = Math.max(0, Math.round(speedKb))
    settings.maxDownloadSpeed = limitKb
    settings.globalSpeedLimitEnabled = enabled
    await invoke('save_all_settings', { settings })
    // Ensure the daemon is up (no-op when aria2 is already running), then push
    // the new limit to new, active, queued and paused downloads immediately.
    await invoke('start_aria2_with_settings', { settings })
    await invoke('apply_global_speed_limit', { settings })
    // Group caps are stored client-side, so re-apply them after the global cap.
    await reapplyAllLimits()
    globalSpeedLimitKb.value = limitKb
    globalSpeedLimitEnabled.value = enabled
    loaded.value = true
  } catch (error) {
    console.error('Failed to update global speed limit:', error)
    throw error
  } finally {
    isApplying.value = false
  }
}

async function toggleGlobalSpeedLimit() {
  if (!loaded.value) {
    await loadGlobalSpeedLimit()
  }
  const nextEnabled = !globalSpeedLimitEnabled.value
  const nextKb = globalSpeedLimitKb.value || 200
  await setGlobalSpeedLimit(nextEnabled, nextKb)
}

export function useGlobalSpeedLimit() {
  return {
    globalSpeedLimitEnabled,
    globalSpeedLimitKb,
    isApplying,
    loaded,
    loadGlobalSpeedLimit,
    toggleGlobalSpeedLimit,
  }
}
