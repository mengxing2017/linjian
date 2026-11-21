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
