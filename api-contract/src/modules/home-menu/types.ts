export type MenuStatus = 'shown' | 'hot' | 'hidden'
export type EnableStatus = 'enabled' | 'pending' | 'disabled'
export interface HomeMenuTag {
  id: string
  name: string
  menu_status: MenuStatus
  source_type: 'admin' | 'user'
  enable_status: EnableStatus
  sort_order: number
  revision: number
  created_at: string
}
export interface HomeMenuSettings {
  latest_label: string
  latest_visible: boolean
  revision: number
  catalog_revision: number
}
export interface HomeMenu {
  id: string
  settings: HomeMenuSettings
  tags: HomeMenuTag[]
}
export interface TagInput {
  name: string
  menu_status: MenuStatus
  enable_status: EnableStatus
  revision?: number
}
export interface HomeMenuQuery {
  search?: string
  menu_status?: MenuStatus
  source_type?: 'admin' | 'user'
  enable_status?: EnableStatus
  page?: number
}
export interface HomeMenuSearch {
  id: string
  tags: HomeMenuTag[]
  page: { number: number; size: number; has_more: boolean }
}
export interface HomeMenuImport {
  id: string
  digest: string
  rows: Array<{
    row: number
    name: string
    menu_status: MenuStatus
    enable_status: EnableStatus
    sort_order: number
  }>
  errors: Array<{ row: number; message: string }>
  expires_at: string
  result: string[] | null
}
