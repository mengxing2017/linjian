import { expect, it, vi, afterEach } from 'vitest'
import { fromBeijingInput, toBeijingInput } from './beijing-time'

afterEach(() => vi.useRealTimers())

it('converts instants to Beijing wall time with the timezone library', () => {
  expect(toBeijingInput('2026-11-19T17:30:00Z')).toBe('2026-11-20T01:30')
  expect(toBeijingInput('2026-11-19T12:30:00-05:00')).toBe('2026-11-20T01:30')
  expect(toBeijingInput('invalid')).toBe('')
  expect(toBeijingInput('')).toBe('')
  expect(toBeijingInput('1990-07-01T00:00:00Z')).toBe('1990-07-01T08:00')
})

it('parses Beijing form values without assuming the browser timezone', () => {
  expect(fromBeijingInput('2026-11-20T01:30')).toBe('2026-11-19T17:30:00.000Z')
  expect(fromBeijingInput('')).toBe('')
})

it('provides the current Beijing minute for new posts', () => {
  vi.useFakeTimers()
  vi.setSystemTime(new Date('2026-11-19T17:30:59Z'))
  expect(toBeijingInput()).toBe('2026-11-20T01:30')
})
