import { describe, expect, it } from 'vitest'
import { ACCENT_STRONG, THEMES, resolveThemeSelection } from '../useTheme'

const LIGHT_BG = '#faf6f0'

function lum(hex: string): number {
  const h = hex.replace('#', '')
  const c = [0, 2, 4].map(i => parseInt(h.slice(i, i + 2), 16) / 255)
  const f = c.map(v => (v <= 0.03928 ? v / 12.92 : Math.pow((v + 0.055) / 1.055, 2.4)))
  return 0.2126 * f[0] + 0.7152 * f[1] + 0.0722 * f[2]
}

function contrast(a: string, b: string): number {
  const l1 = lum(a)
  const l2 = lum(b)
  return (Math.max(l1, l2) + 0.05) / (Math.min(l1, l2) + 0.05)
}

describe('theme selection', () => {
  it('keeps app mode and accent independent', () => {
    expect(THEMES.some(t => t.name === 'linear')).toBe(false)
    expect(THEMES.find(t => t.name === 'indigo')?.accent).toBe('#5e6ad2')

    expect(resolveThemeSelection('gold', 'linear-dark')).toEqual({
      themeName: 'gold',
      mode: 'linear-dark',
    })
    expect(resolveThemeSelection('linear', 'dark')).toEqual({
      themeName: 'indigo',
      mode: 'linear-dark',
    })
    expect(resolveThemeSelection('unknown', 'light')).toEqual({
      themeName: 'default',
      mode: 'light',
    })
  })

  it('every accent has a light-mode strong variant that passes WCAG AA on the light background', () => {
    for (const t of THEMES) {
      const strong = ACCENT_STRONG[t.name]
      expect(strong, `missing ACCENT_STRONG[${t.name}]`).toBeTruthy()
      expect(contrast(strong, LIGHT_BG), `${t.name} ${strong} fails 4.5:1`).toBeGreaterThanOrEqual(4.5)
    }
  })
})