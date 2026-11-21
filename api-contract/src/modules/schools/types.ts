import type { School } from '../business/types'
export type { School } from '../business/types'
export type SchoolDisplay = Pick<
  School,
  | 'id'
  | 'name'
  | 'short_name'
  | 'province'
  | 'city'
  | 'area'
  | 'latitude'
  | 'longitude'
  | 'status'
  | 'logo_url'
  | 'header_bg_image_url'
  | 'theme_primary'
  | 'theme_secondary'
  | 'theme_text_color'
  | 'revision'
>
export interface SchoolInput {
  name: string
  short_name?: string
  province?: string
  city?: string
  area?: string
  latitude?: number | null
  longitude?: number | null
  status?: 'normal' | 'hidden'
  logo_asset_id?: string | null
  header_bg_asset_id?: string | null
  theme_primary?: string
  theme_secondary?: string
  theme_text_color?: string
  revision?: number
  config?: School['config']
}
export interface SchoolFilters {
  page?: number
  search?: string
  province?: string
  city?: string
  area?: string
  status?: 'normal' | 'hidden'
}
