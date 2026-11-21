export interface MiniappUser {
  id: string
  avatar_url: string | null
  avatar_asset_id: string | null
  nickname: string
  account_kind: 'ordinary' | 'simulated'
  mobile: string | null
  school_id: string | null
  school_name: string | null
  post_count: number
  created_at: string
  status: 'normal' | 'banned'
  revision: number
  history?: Array<{
    created_at: string
    actor_id: string
    action: string
    reason: string
    from: string
    to: string
  }>
}
export interface UserFilters {
  account_kind?: 'ordinary' | 'simulated'
  page?: number
  search?: string
  mobile?: string
  status?: string
  sort?: string
  created_from?: string
  created_to?: string
  school_id?: string
  unbound?: boolean
}
