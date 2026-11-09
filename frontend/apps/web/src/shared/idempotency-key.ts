import { customAlphabet, nanoid } from 'nanoid/non-secure'

/** Request deduplication only; works without Web Crypto. Never use for secrets. */
export function createIdempotencyKey(): string {
  return nanoid(32)
}

const uuidHex = customAlphabet('0123456789abcdef', 32)
/** UUID-shaped deduplication key for typed import jobs; not an authentication secret. */
export function createIdempotencyUuid(): string {
  const hex = uuidHex()
  return `${hex.slice(0, 8)}-${hex.slice(8, 12)}-4${hex.slice(13, 16)}-${((parseInt(hex[16], 16) & 3) | 8).toString(16)}${hex.slice(17, 20)}-${hex.slice(20)}`
}
