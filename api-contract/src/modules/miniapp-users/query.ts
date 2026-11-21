import type { UserFilters } from './types'
export function userQuery(filters: UserFilters) {
  const pairs: [string, string][] = [
    ['page[number]', String(filters.page ?? 1)],
    ['page[size]', '30'],
  ]
  for (const key of [
    'account_kind',
    'search',
    'mobile',
    'status',
    'sort',
    'created_from',
    'created_to',
    'school_id',
    'unbound',
  ] as const)
    if (filters[key] !== undefined && filters[key] !== '') pairs.push([key, String(filters[key])])
  return pairs
    .map(([key, value]) => `${encodeURIComponent(key)}=${encodeURIComponent(value)}`)
    .join('&')
}
