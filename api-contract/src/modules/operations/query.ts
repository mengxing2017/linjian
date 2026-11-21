import type { Filters } from './types'
export function operationsQuery(q: Filters = {}) {
  const pairs: string[] = []
  for (const [key, value] of Object.entries(q)) {
    if (value === undefined || value === '') continue
    const name = key === 'page' ? 'page[number]' : key === 'size' ? 'page[size]' : key
    pairs.push(`${encodeURIComponent(name)}=${encodeURIComponent(String(value))}`)
  }
  return pairs.length ? `?${pairs.join('&')}` : ''
}
