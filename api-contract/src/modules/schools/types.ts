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
  latitude?: number
  longitude?: number
  status?: 'normal' | 'hidden'
  logo_asset_id?: string | null
  header_bg_asset_id?: string | null
  theme_primary?: string
  theme_secondary?: string
  theme_text_color?: string
  revision?: number
  config?: School['config']
}
export type SchoolCreateInput = Omit<SchoolInput, 'latitude' | 'longitude'> & { latitude: number; longitude: number }
export type SchoolUpdateInput = Partial<SchoolInput> & { revision: number }
export interface SchoolRecommendationInput {
  latitude: number
  longitude: number
  coordinate_system: 'gcj02'
  scene: 'binding' | 'browsing'
}
export type OnboardingSchoolRecommendationInput = Omit<SchoolRecommendationInput, 'scene'> & { ticket: string }
export type SchoolRecommendation = SchoolDisplay & { distance_m: number }
export interface SchoolFilters {
  page?: number
  search?: string
  province?: string
  city?: string
  area?: string
  status?: 'normal' | 'hidden'
}
