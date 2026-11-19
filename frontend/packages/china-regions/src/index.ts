import data from '../data/regions.json'

export interface RegionNode {
  code: string
  name: string
  children?: RegionNode[]
}

export interface RegionDataset {
  version: string
  source: string
  provinces: RegionNode[]
}

export interface RegionValue {
  province: string
  city: string
  area: string
}

export const regionDataset = data as RegionDataset
export const regionOptions = regionDataset.provinces
export const emptyRegion: RegionValue = { province: '', city: '', area: '' }

export function namesFromCodes(codes: readonly string[]): RegionValue | null {
  if (codes.length === 0) return { ...emptyRegion }
  if (codes.length > 3) return null
  const names: string[] = []
  let nodes = regionOptions
  for (const code of codes) {
    const node = nodes.find((item) => item.code === code)
    if (!node) return null
    names.push(node.name)
    nodes = node.children ?? []
  }
  return {
    province: names[0] ?? '',
    city: names[1] ?? '',
    area: names[2] ?? '',
  }
}

export function codesFromNames(value: RegionValue): string[] | null {
  if (!value.province) return []
  const province = regionOptions.find((item) => item.name === value.province)
  if (!province) return null
  const codes = [province.code]
  if (!value.city) return codes
  const city = province.children?.find((item) => item.name === value.city)
  if (!city) return null
  codes.push(city.code)
  if (!value.area) return codes
  const area = city.children?.find((item) => item.name === value.area)
  if (!area) return null
  codes.push(area.code)
  return codes
}

export function formatRegion(value: RegionValue): string {
  const parts = [value.province, value.city, value.area].filter(Boolean)
  return parts.filter((part, index) => part !== parts[index - 1]).join(' / ')
}
