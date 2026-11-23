export interface MiniappUser {
  id: string
  avatar_url: string | null
  avatar_asset_id: string | null
  nickname: string
  account_kind: 'ordinary' | 'simulated'
  mobile: string | null
  phone_status?: 'unverified' | 'verified' | 'revoked'
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
export interface PhoneBinding {
  id: string
  status: 'unverified' | 'verified' | 'revoked'
  masked_phone: string | null
  verified_at: string | null
  revision: number
}
export interface BindPhoneInput {
  code: string
  request_id: string
  revision: number
  notice_version: 'linjian-phone-v1'
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
