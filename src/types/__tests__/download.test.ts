import { describe, it, expect } from 'vitest'
import { normalizeStatus } from '../download'

describe('normalizeStatus', () => {
  it('normalizes "active"', () => {
    expect(normalizeStatus('active')).toBe('active')
  })

  it('normalizes "waiting"', () => {
    expect(normalizeStatus('waiting')).toBe('waiting')
  })

  it('normalizes "paused"', () => {
    expect(normalizeStatus('paused')).toBe('paused')
  })

  it('normalizes "complete"', () => {
    expect(normalizeStatus('complete')).toBe('complete')
  })

  it('normalizes "error"', () => {
    expect(normalizeStatus('error')).toBe('error')
  })

  it('normalizes "stopped" to "complete"', () => {
    expect(normalizeStatus('stopped')).toBe('complete')
  })

  it('normalizes "removed" to "cancelled"', () => {
    expect(normalizeStatus('removed')).toBe('cancelled')
  })

  it('normalizes "cancelled" pass-through', () => {
    expect(normalizeStatus('cancelled')).toBe('cancelled')
  })

  it('normalizes null to "unknown"', () => {
    expect(normalizeStatus(null)).toBe('unknown')
  })

  it('normalizes undefined to "unknown"', () => {
    expect(normalizeStatus(undefined)).toBe('unknown')
  })

  it('normalizes empty string to "unknown"', () => {
    expect(normalizeStatus('')).toBe('unknown')
  })

  it('normalizes unknown status to "unknown"', () => {
    expect(normalizeStatus('bogus')).toBe('unknown')
  })

  it('handles case-insensitive input', () => {
    expect(normalizeStatus('ACTIVE')).toBe('active')
    expect(normalizeStatus('Active')).toBe('active')
  })
})
