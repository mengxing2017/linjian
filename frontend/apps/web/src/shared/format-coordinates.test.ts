import { expect, it } from 'vitest'
import { formatCoordinates } from './format-coordinates'

it('joins latitude and longitude with a comma', () => {
  expect(formatCoordinates(23.129163, 113.264435)).toBe('23.129163, 113.264435')
  expect(formatCoordinates(23.1, 113.2)).toBe('23.1, 113.2')
})

it('keeps zero coordinates visible', () => {
  expect(formatCoordinates(0, 0)).toBe('0, 0')
})

it('shows a placeholder for missing or incomplete coordinates', () => {
  expect(formatCoordinates(null, null)).toBe('—')
  expect(formatCoordinates(undefined, undefined)).toBe('—')
  expect(formatCoordinates(23.129163, null)).toBe('—')
  expect(formatCoordinates(null, 113.264435)).toBe('—')
})
