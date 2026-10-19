import { computed, h, onBeforeUnmount, onMounted, reactive, ref, watch } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { useI18n } from '../i18n'
import { useGroups } from './useGroups'
import type { AddonManifest, InstalledAddon, YtSearchResult } from '../types/addon'

const thirdParty = ref<InstalledAddon[]>([])
const loaded = ref(false)
const lastError = ref('')

export interface AddonTab {
  id: string
  name: string
  icon: string
  enabled: boolean
  manifest: AddonManifest
}

export function useAddons() {
  const { t } = useI18n()

  async function refresh() {
    try {
      thirdParty.value = await invoke<InstalledAddon[]>('list_addons')
    } catch (e) {
      // Outside Tauri (browser dev) the backend is unavailable.
      lastError.value = String(e)
      thirdParty.value = []
    }
    loaded.value = true
  }

  const tabs = computed<AddonTab[]>(() =>
    thirdParty.value
      .filter((a) => a.enabled)
      .map((a) => ({
        id: a.manifest.id,
        name: a.manifest.name,
        icon: a.manifest.icon || 'extension',
        enabled: a.enabled,
        manifest: a.manifest,
      })),
  )

  const allKnown = computed<AddonTab[]>(() =>
    thirdParty.value.map((a) => ({
      id: a.manifest.id,
      name: a.manifest.name,
      icon: a.manifest.icon || 'extension',
      enabled: a.enabled,
      manifest: a.manifest,
    })),
  )

  const isInstalled = (id: string) => thirdParty.value.some((a) => a.manifest.id === id)

  /** Find an enabled addon tab handling a URL via manifest url_patterns. */
  function matchUrl(url: string): AddonTab | null {
    let host = ''
    try {
      host = new URL(url).hostname.toLowerCase()
    } catch {
      return null
    }
    return tabs.value.find((t) =>
      (t.manifest.url_patterns || []).some((p) => host.includes(p.toLowerCase())),
    ) ?? null
  }

  async function installFromPath(path: string) {
    const installed = await invoke<InstalledAddon>('install_addon_from_path', { path })
    await refresh()
    return installed
  }

  async function installFromUrl(url: string) {
    const installed = await invoke<InstalledAddon>('install_addon_from_url', { url })
    await refresh()
    return installed
  }

  async function installFromDir(path: string) {
    const installed = await invoke<InstalledAddon>('install_addon_from_dir', { path })
    await refresh()
    return installed
  }

  async function exportTemplate(dir: string) {
    return invoke<string>('export_addon_template', { dir })
  }

  async function exportPackage(id: string, dir: string) {
    return invoke<string>('export_addon_package', { id, dir })
  }

  async function uninstall(id: string) {
    const res = await invoke<string>('uninstall_addon', { id })
    await refresh()
    return res
  }

  async function setEnabled(id: string, enabled: boolean) {
    await invoke<string>('set_addon_enabled', { id, enabled })
    await refresh()
  }

  async function fetchUrl(url: string): Promise<string> {
    return invoke<string>('addon_fetch_url', { url })
  }

  async function ytSearch(query: string): Promise<YtSearchResult> {
    return invoke<YtSearchResult>('addon_yt_search', { query })
  }

  async function readEntry(id: string): Promise<string> {
    return invoke<string>('addon_read_entry', { id })
  }

  /** Host API injected into community addon.js modules. */
  function hostApi(notify: (msg: string, type: 'success' | 'error' | 'info') => void) {
    const { createGroup, addMember } = useGroups()
    return {
      h,
      ref,
      reactive,
      computed,
      watch,
      onMounted,
      onBeforeUnmount,
      t,
      notify,
      invoke,
      fetchUrl,
      ytSearch,
      createGroup: (name: string, color?: string) => {
        const g = createGroup(name, color || '#1DB954')
        window.dispatchEvent(new Event('moonhunt:groups-changed'))
        return { id: g.id, name: g.name }
      },
      addToGroup: async (groupId: string, gid: string) => {
        const g = await addMember(groupId, gid)
        window.dispatchEvent(new Event('moonhunt:groups-changed'))
        return !!g
      },
      // Same effect as pressing Cancel on a download in the Downloads page.
      // Only call for downloads that are still active (see get_all_downloads
      // statuses); completed rows must never be passed here.
      cancelDownload: async (gid: string) => {
        if (typeof gid === 'string' && gid.startsWith('yt-dlp-direct-')) {
          await invoke('cancel_yt_download', { gid })
        } else {
          await invoke('remove_download', { gid })
        }
        return true
      },
    }
  }

  return {
    tabs,
    allKnown,
    thirdParty,
    loaded,
    lastError,
    refresh,
    isInstalled,
    matchUrl,
    installFromPath,
    installFromUrl,
    installFromDir,
    exportTemplate,
    exportPackage,
    uninstall,
    setEnabled,
    fetchUrl,
    ytSearch,
    readEntry,
    hostApi,
  }
}
