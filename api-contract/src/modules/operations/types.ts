import type { ImageRef } from '../product-workflows/client'
import type { BannerAction, EffectiveService } from './services'
export type Module = 'group-qrcodes' | 'home-banners'
export type FeedbackType = 'product_suggestion' | 'bug_report' | 'other'
export interface OperationInput {
  revision: number
  title: string
  content: string
  image_ref: ImageRef
  action: BannerAction
  active: boolean
  sort_order: number
  starts_at: string | null
  ends_at: string | null
  popup_strategy: 'always' | 'daily' | 'interval'
  popup_interval_days: number
}
export interface OperationResource extends OperationInput {
  id: string
  school_id: string | null
  school_name: string
  kind: 'qrcode' | 'banner'
  image_url: string | null
  views: number
  clicks: number
  action_available?: boolean
  needs_repair?: boolean
}
export interface CustomerInput {
  revision: number
  enabled: boolean
  business_asset_id: string | null
  join_asset_id: string | null
  service_asset_id: string | null
  fallback_asset_id: string | null
  note: string
}
export interface CustomerConfig extends CustomerInput {
  id: string
}
export interface FeedbackInput {
  feedback_type: FeedbackType
  content: string
  contact: string
  images: string[]
  submission_key: string
}
export interface Feedback {
  id: string
  school_id: string
  account_id: string
  reporter_name: string
  school_name: string
  feedback_type: FeedbackType | null
  content: string
  contact: string
  images: string[]
  state: 'open' | 'resolved' | 'dismissed'
  submitted_ip: string | null
  revision: number
  created_at: string
  updated_at: string
}
export interface Display {
  id: string
  server_time: string
  qrcode: OperationResource | null
  banners: OperationResource[]
  shortcuts: EffectiveService[]
  categories: { id: string; name: string }[]
  config_token: string
  customer: {
    enabled: boolean
    note?: string
    business?: string | null
    join?: string | null
    service?: string | null
  }
}
export interface Filters {
  page?: number
  size?: number
  search?: string
  status?: string
  strategy?: string
  action?: string
  feedback_type?: string
  contact?: string
  user?: string
  starts_at?: string
  ends_at?: string
  defaults?: boolean
}
