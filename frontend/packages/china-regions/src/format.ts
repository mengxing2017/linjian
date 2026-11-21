import type { RegionValue } from './types'
export function formatRegion(value: RegionValue): string {
  const parts = [value.province, value.city, value.area].filter(Boolean)
  return parts.filter((part, index) => part !== parts[index - 1]).join(' / ')
}
