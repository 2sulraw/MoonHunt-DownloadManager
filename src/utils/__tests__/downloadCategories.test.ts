import { describe, it, expect } from 'vitest'
import {
  formattedCategoryFolder,
  joinDownloadPath,
  packagedCategoryFolder,
  resolveCategoryDir,
  sanitizeCategoryFolder,
} from '../downloadCategories'

describe('formattedCategoryFolder', () => {
  it('maps example formats to their own folders', () => {
    expect(formattedCategoryFolder('clip.mp4')).toBe('mp4')
    expect(formattedCategoryFolder('setup.exe')).toBe('exe')
    expect(formattedCategoryFolder('archive.zip')).toBe('zip')
  })

  it('groups other video and file formats', () => {
    expect(formattedCategoryFolder('movie.mkv')).toBe('video')
    expect(formattedCategoryFolder('manual.pdf')).toBe('document')
  })

  it('handles URLs and unknown extensions', () => {
    expect(formattedCategoryFolder('https://example.com/files/song.mp4?download=1')).toBe('mp4')
    expect(formattedCategoryFolder('no-extension')).toBe('other')
  })
})

describe('packagedCategoryFolder', () => {
  it('groups formats into broad trees instead of per-format folders', () => {
    expect(packagedCategoryFolder('clip.mp4')).toBe('video')
    expect(packagedCategoryFolder('movie.mkv')).toBe('video')
    expect(packagedCategoryFolder('song.aiff')).toBe('audio')
    expect(packagedCategoryFolder('archive.zip')).toBe('archive')
    expect(packagedCategoryFolder('archive.rar')).toBe('archive')
    expect(packagedCategoryFolder('setup.exe')).toBe('installer')
  })

  it('handles URLs and unknown extensions', () => {
    expect(packagedCategoryFolder('https://example.com/files/song.mp4?download=1')).toBe('video')
    expect(packagedCategoryFolder('no-extension')).toBe('other')
  })
})

describe('sanitizeCategoryFolder', () => {
  it('keeps simple folder names', () => {
    expect(sanitizeCategoryFolder('  My Downloads  ')).toBe('My Downloads')
  })

  it('replaces invalid path characters', () => {
    expect(sanitizeCategoryFolder('A/B:C*D')).toBe('A_B_C_D')
  })

  it('falls back for empty names', () => {
    expect(sanitizeCategoryFolder('')).toBe('uncategorized')
  })
})

describe('joinDownloadPath', () => {
  it('joins with a single separator', () => {
    expect(joinDownloadPath('C:\\Downloads\\', 'mp4')).toBe('C:\\Downloads\\mp4')
    expect(joinDownloadPath('C:/Downloads', 'zip')).toBe('C:/Downloads\\zip')
  })
})

describe('resolveCategoryDir', () => {
  const base = { enabled: true, mode: 'formatted' as const, selected: '', filename: '', baseDir: 'C:\\Downloads' }

  it('returns the base folder when categories are disabled', () => {
    expect(resolveCategoryDir({ ...base, enabled: false })).toBe('C:\\Downloads')
  })

  it('requires a selected manual category', () => {
    expect(resolveCategoryDir({ ...base, mode: 'manual', selected: '' })).toBeNull()
    expect(resolveCategoryDir({ ...base, mode: 'manual', selected: 'Games' })).toBe('C:\\Downloads\\Games')
  })

  it('resolves the formatted folder', () => {
    expect(resolveCategoryDir({ ...base, filename: 'setup.exe' })).toBe('C:\\Downloads\\exe')
  })

  it('resolves the packaged tree folder', () => {
    expect(resolveCategoryDir({ ...base, mode: 'packaged', filename: 'setup.exe' })).toBe('C:\\Downloads\\installer')
    expect(resolveCategoryDir({ ...base, mode: 'packaged', filename: 'clip.mp4' })).toBe('C:\\Downloads\\video')
  })
})
