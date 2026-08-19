import { describe, it, expect } from 'vitest'
import {
  formatBytes,
  formatFileSize,
  formatSpeed,
  calculateRemainingSeconds,
  formatRemainingTime,
  formatDateTime,
  parseFormattedSize,
  parseDownloadSize,
} from '../format'

describe('formatBytes', () => {
  it('returns "0 B" for zero', () => {
    expect(formatBytes(0)).toBe('0 B')
  })

  it('returns "0 B" for negative', () => {
    expect(formatBytes(-100)).toBe('0 B')
  })

  it('returns "0 B" for NaN', () => {
    expect(formatBytes(NaN)).toBe('0 B')
  })

  it('formats bytes correctly', () => {
    expect(formatBytes(500)).toBe('500 B')
  })

  it('formats kilobytes correctly', () => {
    expect(formatBytes(1536)).toBe('1.5 KB')
  })

  it('formats megabytes correctly', () => {
    expect(formatBytes(1048576)).toBe('1.0 MB')
  })

  it('formats gigabytes correctly', () => {
    expect(formatBytes(1073741824)).toBe('1.0 GB')
  })

  it('formats terabytes correctly', () => {
    expect(formatBytes(1099511627776)).toBe('1.0 TB')
  })
})

describe('formatFileSize', () => {
  it('is an alias for formatBytes', () => {
    expect(formatFileSize(1024)).toBe(formatBytes(1024))
  })
})

describe('parseFormattedSize', () => {
  it('returns 0 for empty input', () => {
    expect(parseFormattedSize('')).toBe(0)
    expect(parseFormattedSize(null)).toBe(0)
  })

  it('parses common size units', () => {
    expect(parseFormattedSize('500 B')).toBe(500)
    expect(parseFormattedSize('1.5 KB')).toBe(1536)
    expect(parseFormattedSize('34.0 MB')).toBe(34 * 1048576)
    expect(parseFormattedSize('6.0 GB')).toBe(6 * 1073741824)
  })
})

describe('parseDownloadSize', () => {
  it('parses a plain completed size', () => {
    expect(parseDownloadSize('34.0 MB')).toBe(34 * 1048576)
  })

  it('prefers the total side of a completed / total value', () => {
    expect(parseDownloadSize('1.0 MB / 6.0 GB')).toBe(6 * 1073741824)
  })

  it('handles zero and missing values', () => {
    expect(parseDownloadSize('0')).toBe(0)
    expect(parseDownloadSize('')).toBe(0)
  })
})

describe('formatSpeed', () => {
  it('returns "0 B/s" for zero', () => {
    expect(formatSpeed(0)).toBe('0 B/s')
  })

  it('returns "0 B/s" for negative', () => {
    expect(formatSpeed(-1)).toBe('0 B/s')
  })

  it('formats speed correctly', () => {
    expect(formatSpeed(1024)).toBe('1.0 KB/s')
  })

  it('formats large speeds correctly', () => {
    expect(formatSpeed(1048576)).toBe('1.0 MB/s')
  })
})

describe('calculateRemainingSeconds', () => {
  it('returns undefined when there is no speed', () => {
    expect(calculateRemainingSeconds(1000, 100, 0)).toBeUndefined()
  })

  it('returns undefined when already complete', () => {
    expect(calculateRemainingSeconds(1000, 1000, 100)).toBeUndefined()
  })

  it('rounds the estimate up', () => {
    expect(calculateRemainingSeconds(1000, 100, 300)).toBe(3)
  })
})

describe('formatRemainingTime', () => {
  it('returns "—" for unknown values', () => {
    expect(formatRemainingTime(undefined)).toBe('—')
    expect(formatRemainingTime(null)).toBe('—')
    expect(formatRemainingTime(0)).toBe('—')
  })

  it('formats seconds, minutes, hours, and days', () => {
    expect(formatRemainingTime(45)).toBe('45s')
    expect(formatRemainingTime(750)).toBe('12m 30s')
    expect(formatRemainingTime(3900)).toBe('1h 5m')
    expect(formatRemainingTime(90000)).toBe('1d 1h')
  })
})

describe('formatDateTime', () => {
  it('returns "—" for null', () => {
    expect(formatDateTime(null)).toBe('—')
  })

  it('returns "—" for empty string', () => {
    expect(formatDateTime('')).toBe('—')
  })

  it('formats a timestamp correctly', () => {
    const result = formatDateTime(1700000000000)
    expect(result).not.toBe('—')
    expect(typeof result).toBe('string')
  })

  it('formats a date string correctly', () => {
    const result = formatDateTime('2024-01-15')
    expect(result).not.toBe('—')
  })
})
