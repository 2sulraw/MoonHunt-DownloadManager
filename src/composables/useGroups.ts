/**
 * Canonical owner of download groups + per-group speed limits.
 * All UI (Downloads page, context menu, settings reapply) must go through this.
 */
import { ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'

export interface DownloadGroup {
  id: string
  name: string
  color: string
  memberGids: string[]
  onComplete: string
  collapsed: boolean
  /** Per-group speed cap in KB/s; 0 = unlimited */
  speedLimit: number
  /** Per-group cumulative download byte limit */
  downloadLimitEnabled: boolean
  downloadLimitBytes: number
  downloadLimitUsedBytes: number
  /** Epoch ms when the group was created; used for date-added sorting. */
  added_at?: number
}

const STORAGE_KEY = 'moonhunt_groups'

function groupAddedAtFromId(id: string): number {
  const match = /^g-(\d+)/.exec(id)
  return match ? Number(match[1]) : 0
}

function loadGroups(): DownloadGroup[] {
  try {
    const raw = JSON.parse(localStorage.getItem(STORAGE_KEY) || '[]') as any[]
    return raw.map((g) => normalizeGroup(g))
  } catch {
    return []
  }
}

function normalizeGroup(g: Partial<DownloadGroup> & Record<string, unknown>): DownloadGroup {
  return {
    id: String(g.id ?? `g-${Date.now()}`),
    name: String(g.name || 'Group'),
    color: String(g.color || '#ef5350'),
    memberGids: Array.isArray(g.memberGids) ? g.memberGids.map(String) : [],
    onComplete: (typeof g.onComplete === 'string' ? g.onComplete : 'none'),
    collapsed: Boolean(g.collapsed),
    speedLimit: typeof g.speedLimit === 'number' ? g.speedLimit : 0,
    downloadLimitEnabled: Boolean(g.downloadLimitEnabled),
    downloadLimitBytes: typeof g.downloadLimitBytes === 'number' ? g.downloadLimitBytes : 0,
    downloadLimitUsedBytes: typeof g.downloadLimitUsedBytes === 'number' ? g.downloadLimitUsedBytes : 0,
    added_at: typeof g.added_at === 'number' && g.added_at > 0 ? g.added_at : groupAddedAtFromId(String(g.id ?? '')),
  }
}

async function applySpeedLimitKb(gid: string, speedLimitKB: number): Promise<void> {
  const speedBytes = speedLimitKB > 0 ? speedLimitKB * 1024 : 0
  await invoke('apply_download_speed_limit', { gid, speedLimit: speedBytes })
}

// Module-level singleton so layout / settings / downloads page share one store
const groups = ref<DownloadGroup[]>(loadGroups())

function saveGroups() {
  localStorage.setItem(STORAGE_KEY, JSON.stringify(groups.value))
}

function reloadGroups() {
  groups.value = loadGroups()
}

export function useGroups() {

  function createGroup(
    name: string,
    color: string,
    memberGids: string[] = [],
    onComplete = 'none',
    speedLimit = 0,
    downloadLimitEnabled = false,
    downloadLimitBytes = 0,
  ): DownloadGroup {
    const g: DownloadGroup = {
      id: `g-${Date.now()}-${Math.random().toString(36).slice(2, 7)}`,
      name: name.trim() || 'New Group',
      color,
      memberGids: [...memberGids],
      onComplete,
      collapsed: false,
      speedLimit: speedLimit > 0 ? speedLimit : 0,
      downloadLimitEnabled,
      downloadLimitBytes: downloadLimitBytes > 0 ? downloadLimitBytes : 0,
      downloadLimitUsedBytes: 0,
      added_at: Date.now(),
    }
    groups.value.push(g)
    saveGroups()
    return g
  }

  function updateGroup(id: string, patch: Partial<DownloadGroup>) {
    const g = groups.value.find((x) => x.id === id)
    if (!g) return
    Object.assign(g, patch)
    saveGroups()
  }

  function deleteGroup(id: string): DownloadGroup | undefined {
    const g = groups.value.find((x) => x.id === id)
    groups.value = groups.value.filter((x) => x.id !== id)
    saveGroups()
    return g
  }

  async function applyMemberLimit(gid: string, speedLimitKB: number): Promise<void> {
    try {
      await applySpeedLimitKb(gid, speedLimitKB)
    } catch (e) {
      console.error('Failed to apply group speed limit:', gid, e)
    }
  }

  async function applyGroupLimits(group: DownloadGroup): Promise<void> {
    const limit = group.speedLimit || 0
    await Promise.all(
      (group.memberGids || []).map((gid) => applyMemberLimit(gid, limit)),
    )
  }

  /** Re-apply every group's speed limits (after aria2 restart / set_proxy). */
  async function reapplyAllLimits(): Promise<void> {
    reloadGroups()
    for (const g of groups.value) {
      if (g.speedLimit > 0 && g.memberGids.length) {
        await applyGroupLimits(g)
      }
    }
  }

  async function addMember(groupId: string, gid: string): Promise<DownloadGroup | undefined> {
    const g = groups.value.find((x) => x.id === groupId)
    if (!g) return undefined
    await applyMemberLimit(gid, g.speedLimit || 0)
    if (!g.memberGids.includes(gid)) {
      g.memberGids.push(gid)
      saveGroups()
    }
    return g
  }

  async function removeMember(groupId: string, gid: string): Promise<DownloadGroup | undefined> {
    const g = groups.value.find((x) => x.id === groupId)
    if (!g) return undefined
    g.memberGids = g.memberGids.filter((id) => id !== gid)
    saveGroups()
    // Clear group-imposed cap when leaving the group
    await applyMemberLimit(gid, 0)
    return g
  }

  function reorderGroups(oldIndex: number, newIndex: number) {
    const [item] = groups.value.splice(oldIndex, 1)
    groups.value.splice(newIndex, 0, item)
    saveGroups()
  }

  function findGroup(id: string): DownloadGroup | undefined {
    return groups.value.find((g) => g.id === id)
  }

  function resetGroupDownloadLimit(groupId: string) {
    const g = groups.value.find((x) => x.id === groupId)
    if (!g) return
    g.downloadLimitUsedBytes = 0
    saveGroups()
  }

  return {
    groups,
    saveGroups,
    reloadGroups,
    createGroup,
    updateGroup,
    deleteGroup,
    addMember,
    removeMember,
    reorderGroups,
    applyMemberLimit,
    applyGroupLimits,
    reapplyAllLimits,
    findGroup,
    resetGroupDownloadLimit,
  }
}
