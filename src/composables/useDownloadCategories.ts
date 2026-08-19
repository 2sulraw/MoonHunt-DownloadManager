import { ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { sanitizeCategoryFolder } from '../utils/downloadCategories'

const STORAGE_KEY = 'moonhunt_download_categories'

function normalizeCategories(raw: unknown): string[] {
  if (!Array.isArray(raw)) return []
  const seen = new Set<string>()
  const result: string[] = []
  for (const item of raw) {
    if (typeof item !== 'string') continue
    const folder = sanitizeCategoryFolder(item)
    if (!folder || seen.has(folder)) continue
    seen.add(folder)
    result.push(folder)
  }
  return result
}

function loadLocalCategories(): string[] {
  try {
    return normalizeCategories(JSON.parse(localStorage.getItem(STORAGE_KEY) || '[]'))
  } catch {
    return []
  }
}

const categories = ref<string[]>(loadLocalCategories())
let loadPromise: Promise<void> | null = null

function persistToBackend() {
  try {
    void invoke('save_download_categories', { categories: [...categories.value] }).catch(() => {})
  } catch {
    // Running outside Tauri; localStorage still keeps the categories.
  }
}

async function loadFromBackend() {
  try {
    const saved = await invoke<string[] | null>('get_download_categories')
    if (saved === null || saved === undefined) {
      if (categories.value.length) persistToBackend()
      return
    }
    categories.value = normalizeCategories(saved)
    localStorage.setItem(STORAGE_KEY, JSON.stringify(categories.value))
  } catch {
    // Keep the localStorage fallback when Tauri is unavailable.
  }
}

export function loadDownloadCategories(): Promise<void> {
  if (!loadPromise) {
    loadPromise = loadFromBackend().finally(() => {
      loadPromise = null
    })
  }
  return loadPromise
}

export function useDownloadCategories() {
  function saveLocal() {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(categories.value))
  }

  function createCategory(name: string): string {
    const folder = sanitizeCategoryFolder(name)
    if (!folder || categories.value.includes(folder)) return folder
    categories.value.push(folder)
    saveLocal()
    persistToBackend()
    return folder
  }

  function renameCategory(oldName: string, newName: string): string {
    const folder = sanitizeCategoryFolder(newName)
    const index = categories.value.indexOf(oldName)
    if (!folder || index === -1) return ''
    if (categories.value.includes(folder)) return folder === oldName ? folder : ''
    categories.value[index] = folder
    saveLocal()
    persistToBackend()
    return folder
  }

  function removeCategory(name: string): boolean {
    const index = categories.value.indexOf(name)
    if (index === -1) return false
    categories.value.splice(index, 1)
    saveLocal()
    persistToBackend()
    return true
  }

  return {
    categories,
    createCategory,
    renameCategory,
    removeCategory,
  }
}
