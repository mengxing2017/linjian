import { afterEach, expect, it, vi } from 'vitest'
import { createIdempotencyKey, createIdempotencyUuid } from './idempotency-key'

const requestKey = /^[A-Za-z0-9_-]{32}$/
afterEach(() => vi.unstubAllGlobals())

it.each([undefined, {}])('generates distinct request keys without Web Crypto (%s)', (crypto) => {
  vi.stubGlobal('crypto', crypto)
  const keys = Array.from({ length: 1024 }, createIdempotencyKey)
  expect(keys.every((key) => requestKey.test(key))).toBe(true)
  expect(new Set(keys).size).toBe(keys.length)
})

it('does not call native crypto methods even when they are present', () => {
  const unsupported = vi.fn(() => {
    throw new Error('Web Crypto must not be used')
  })
  vi.stubGlobal('crypto', { randomUUID: unsupported, getRandomValues: unsupported })
  expect(createIdempotencyKey()).toMatch(requestKey)
  expect(unsupported).not.toHaveBeenCalled()
})

it('generates unique UUID import keys without requiring a secure browser context', () => {
  vi.stubGlobal('crypto', undefined)
  const keys = Array.from({ length: 1024 }, createIdempotencyUuid)
  expect(
    keys.every((key) =>
      /^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/.test(key),
    ),
  ).toBe(true)
  expect(new Set(keys).size).toBe(keys.length)
})
