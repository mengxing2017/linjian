import { describe, expect, it } from 'vitest'
import { formatDate } from './format-date'

describe('localized date display', () => {
  it('normalizes timezone offsets and displays the same instant in local time', () => {
    const expected = new Date('2026-10-21T00:00:00Z').toLocaleString('zh-CN')
    expect(formatDate('2026-10-21T00:00:00Z')).toBe(expected)
    expect(formatDate('2026-10-21T08:00:00+08:00')).toBe(expected)
    expect(formatDate('2026-10-21T08:00:00+08')).toBe(expected)
  })
  it('handles missing and invalid values without displaying Invalid Date', () => {
    for (const value of [undefined, null, '', ' ', 'invalid', {}, 123]) {
      expect(formatDate(value)).toBe('-')
    }
    expect(formatDate(null, '尚未同步')).toBe('尚未同步')
  })
})
