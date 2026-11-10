import { expect, it } from 'vitest'
import { shouldPopup, popupKey } from '../src/features/operations/policy'
it('uses Shanghai natural days and elapsed interval days independently of local timezone', () => {
  const before = Date.parse('2026-11-10T15:59:00Z'), after = Date.parse('2026-11-10T16:01:00Z')
  expect(shouldPopup({ popup_strategy: 'daily', popup_interval_days: 1 }, before, after)).toBe(true)
  expect(shouldPopup({ popup_strategy: 'interval', popup_interval_days: 1 }, before, after)).toBe(false)
  expect(shouldPopup({ popup_strategy: 'interval', popup_interval_days: 2 }, before, before + 2 * 86400000)).toBe(true)
  expect(shouldPopup({ popup_strategy: 'daily', popup_interval_days: 1 }, after, after + 60000)).toBe(false)
  expect(popupKey('user-a', 'school-a', 'resource')).not.toBe(popupKey('user-b', 'school-a', 'resource'))
  expect(popupKey('user-a', 'school-a', 'resource')).not.toBe(popupKey('user-a', 'school-b', 'resource'))
})
