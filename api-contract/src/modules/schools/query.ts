import type { SchoolFilters } from './types'
export const schoolQuery = (filters: SchoolFilters = {}) => {
  const { page, ...rest } = filters
  const pairs = [
    ['page[number]', String(page ?? 1)],
    ['page[size]', '30'],
  ]
  for (const [k, v] of Object.entries(rest))
    if (v !== undefined && v !== '') pairs.push([k, String(v)])
  return '?' + pairs.map(([k, v]) => `${encodeURIComponent(k)}=${encodeURIComponent(v)}`).join('&')
}
