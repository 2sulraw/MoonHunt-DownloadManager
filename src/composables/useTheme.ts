import { ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import type { Settings } from '../types/settings'

export interface ThemeDef {
  name: string
  label: string
  accent: string
}

export type ThemeMode = 'dark' | 'light' | 'linear-dark'

export const THEMES: ThemeDef[] = [
  { name: 'default', label: 'Default', accent: '#4fc3f7' },
  { name: 'cyan', label: 'Cyan', accent: '#1e88e5' },
  { name: 'purple', label: 'Purple', accent: '#b388ff' },
  { name: 'green', label: 'Green', accent: '#81c784' },
  { name: 'red', label: 'Red', accent: '#ef5350' },
  { name: 'gold', label: 'Gold', accent: '#ffd54f' },
  { name: 'orange', label: 'Orange', accent: '#ff8a65' },
  { name: 'pink', label: 'Pink', accent: '#f06292' },
  { name: 'gray', label: 'Gray', accent: '#9e9e9e' },
  { name: 'lemon', label: 'Lemon', accent: '#cddc39' },
  { name: 'aqua', label: 'Aqua', accent: '#26c6da' },
  { name: 'persian-blue', label: 'Persian Blue', accent: '#57c5c6' },
  { name: 'teal', label: 'Teal', accent: '#26a69a' },
  { name: 'lavender', label: 'Lavender', accent: '#9575cd' },
  { name: 'coral', label: 'Coral', accent: '#ff7043' },
  { name: 'indigo', label: 'Indigo', accent: '#5e6ad2' },
]

// Darkened accent variants for light mode. The base accents are pastel and
// fail WCAG contrast (1.3-3.4:1) as text on the near-white light background;
// these keep the hue but reach >=4.5:1. Swapped into --accent-strong.
export const ACCENT_STRONG: Record<string, string> = {
  default: '#0078ad',
  cyan: '#0971cd',
  purple: '#7d33a8',
  green: '#328135',
  red: '#d20805',
  gold: '#8a6900',
  orange: '#d63300',
  pink: '#d1054a',
  gray: '#6b6b6b',
  lemon: '#6a730c',
  aqua: '#0e7d8b',
  'persian-blue': '#227b7c',
  teal: '#157f75',
  lavender: '#7a4dd2',
  coral: '#d63300',
  indigo: '#2634b0',
}

export function normalizeThemeName(themeName: string): string {
  if (themeName === 'linear') return 'indigo'
  return THEMES.some(t => t.name === themeName) ? themeName : 'default'
}

export function normalizeThemeMode(mode: string): ThemeMode {
  return mode === 'light' || mode === 'linear-dark' ? mode : 'dark'
}

export function resolveThemeSelection(theme: string, mode: string): { themeName: string; mode: ThemeMode } {
  if (theme === 'linear') {
    // Older versions stored Linear Dark as an accent named "linear".
    return { themeName: 'indigo', mode: 'linear-dark' }
  }
  return { themeName: normalizeThemeName(theme), mode: normalizeThemeMode(mode) }
}

const currentTheme = ref<string>('default')
const currentMode = ref<ThemeMode>('dark')

export function useTheme() {
  function hexToRgb(hex: string): string {
    const h = hex.replace('#', '')
    const r = parseInt(h.substring(0, 2), 16)
    const g = parseInt(h.substring(2, 4), 16)
    const b = parseInt(h.substring(4, 6), 16)
    return `${r}, ${g}, ${b}`
  }

  function applyAccent(themeName: string) {
    const theme = THEMES.find(t => t.name === normalizeThemeName(themeName)) || THEMES[0]
    const root = document.documentElement
    root.style.setProperty('--accent', theme.accent)
    root.style.setProperty('--accent-rgb', hexToRgb(theme.accent))
    root.dataset.accent = theme.name
  }

  function applyMode(mode: ThemeMode) {
    document.documentElement.classList.remove('light', 'linear-dark')
    if (mode === 'linear-dark') {
      document.documentElement.classList.add('linear-dark')
    } else if (mode === 'light') {
      document.documentElement.classList.add('light')
    }
  }

  function applyTheme(themeName: string, mode: ThemeMode) {
    currentTheme.value = normalizeThemeName(themeName)
    currentMode.value = normalizeThemeMode(mode)
    applyAccent(currentTheme.value)
    applyMode(currentMode.value)
    // Accent text/icons need a darker shade on the near-white light surfaces;
    // on dark surfaces the bright base accent already reads fine.
    const theme = THEMES.find(t => t.name === currentTheme.value) || THEMES[0]
    const strong = currentMode.value === 'light' ? (ACCENT_STRONG[theme.name] ?? theme.accent) : theme.accent
    document.documentElement.style.setProperty('--accent-strong', strong)
  }

  async function saveAndApply(themeName: string, mode?: ThemeMode) {
    applyTheme(themeName, mode ?? currentMode.value)
    try {
      const settings = await invoke<Settings>('get_all_settings')
      if (settings) {
        settings.theme = currentTheme.value
        settings.themeMode = currentMode.value
        await invoke('save_all_settings', { settings })
      }
    } catch { /* silent */ }
  }

  async function toggleMode() {
    const newMode: ThemeMode = currentMode.value === 'light' ? 'dark' : 'light'
    await saveAndApply(currentTheme.value, newMode)
  }

  async function loadTheme() {
    try {
      const settings = await invoke<Settings>('get_all_settings')
      const selection = resolveThemeSelection(settings?.theme || 'default', settings?.themeMode || 'dark')
      applyTheme(selection.themeName, selection.mode)
    } catch {
      applyTheme('default', 'dark')
    }
  }

  return {
    currentTheme,
    currentMode,
    THEMES,
    applyMode,
    applyTheme,
    saveAndApply,
    toggleMode,
    loadTheme,
  }
}
