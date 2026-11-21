import data from '../data/regions.json'
import type { RegionDataset, RegionValue } from './types'
export const regionDataset = data as RegionDataset
export const regionOptions = regionDataset.provinces
export const emptyRegion: RegionValue = { province: '', city: '', area: '' }
