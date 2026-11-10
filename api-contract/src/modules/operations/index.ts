import type { Client, Cancellation } from '../../http/client'
import type { ImageRef, ResourceAction } from '../product-workflows/client'
export type Module = 'group-qrcodes' | 'home-banners'
export type FeedbackType = 'product_suggestion' | 'bug_report' | 'other'
export interface OperationInput {
  revision: number; title: string; content: string; image_ref: ImageRef; action: ResourceAction
  active: boolean; sort_order: number; starts_at: string | null; ends_at: string | null
  popup_strategy: 'always' | 'daily' | 'interval'; popup_interval_days: number
}
export interface OperationResource extends OperationInput {
  id: string; school_id: string | null; school_name: string; kind: 'qrcode' | 'banner'
  image_url: string | null; views: number; clicks: number; action_available?: boolean; needs_repair?: boolean
}
export interface CustomerInput {
  revision: number; enabled: boolean; business_asset_id: string | null; join_asset_id: string | null
  service_asset_id: string | null; fallback_asset_id: string | null; note: string
}
export interface CustomerConfig extends CustomerInput { id: string }
export interface FeedbackInput {
  feedback_type: FeedbackType; content: string; contact: string; images: string[]; submission_key: string
}
export interface Feedback {
  id: string; school_id: string; account_id: string; reporter_name: string; school_name: string
  feedback_type: FeedbackType | null; content: string; contact: string; images: string[]
  state: 'open' | 'resolved' | 'dismissed'; submitted_ip: string | null; revision: number
  created_at: string; updated_at: string
}
export interface Display {
  id: string; server_time: string; qrcode: OperationResource | null; banners: OperationResource[]
  customer: { enabled: boolean; note?: string; business?: string | null; join?: string | null; service?: string | null }
}
export interface Filters {
  page?: number; size?: number; search?: string; status?: string; strategy?: string; action?: string
  feedback_type?: string; contact?: string; user?: string; starts_at?: string; ends_at?: string; defaults?: boolean
}
export function operationsQuery(q: Filters = {}) {
  const pairs: string[] = []
  for (const [key, value] of Object.entries(q)) {
    if (value === undefined || value === '') continue
    const name = key === 'page' ? 'page[number]' : key === 'size' ? 'page[size]' : key
    pairs.push(`${encodeURIComponent(name)}=${encodeURIComponent(String(value))}`)
  }
  return pairs.length ? `?${pairs.join('&')}` : ''
}
export function createOperationsClient(c: Client) {
  const root = (s?: string) => s ? `/api/v1/schools/${encodeURIComponent(s)}` : '/api/v1/platform'
  return {
    list: (s: string | undefined, m: Module, q: Filters = {}, signal?: Cancellation) => c.requestCollection<OperationResource>(`${root(s)}/${m}${operationsQuery(q)}`, signal),
    get: (s: string | undefined, m: Module, id: string) => c.requestResource<OperationResource>(`${root(s)}/${m}/${encodeURIComponent(id)}`),
    save: (s: string | undefined, m: Module, v: OperationInput, id?: string) => c.requestResource<OperationResource>(`${root(s)}/${m}${id ? `/${encodeURIComponent(id)}` : ''}`, id ? 'PATCH' : 'POST', v),
    remove: (s: string | undefined, m: Module, id: string, revision: number) => c.requestResource<null>(`${root(s)}/${m}/${encodeURIComponent(id)}`, 'DELETE', { revision }),
    customer: () => c.requestResource<CustomerConfig>('/api/v1/platform/customer-config'),
    saveCustomer: (v: CustomerInput) => c.requestResource<CustomerConfig>('/api/v1/platform/customer-config', 'PATCH', v),
    feedback: (s: string, q: Filters = {}, signal?: Cancellation) => c.requestCollection<Feedback>(`${root(s)}/feedback${operationsQuery(q)}`, signal),
    feedbackDetail: (s: string, id: string) => c.requestResource<Feedback>(`${root(s)}/feedback/${encodeURIComponent(id)}`),
    deleteFeedback: (s: string, id: string, revision: number) => c.requestResource<null>(`${root(s)}/feedback/${encodeURIComponent(id)}`, 'DELETE', { revision }),
    submit: (s: string, v: FeedbackInput) => c.requestResource<{ id: string; submitted: boolean }>(`${root(s)}/feedback`, 'POST', v),
    display: (s: string) => c.requestResource<Display>(`${root(s)}/operations-display`),
    event: (s: string, v: { event_id: string; resource_id: string; event_type: 'view' | 'click'; display_cycle: string; occurred_at: string }) => c.requestResource<{ id: string; accepted: boolean }>(`${root(s)}/banner-events`, 'POST', v),
  }
}
