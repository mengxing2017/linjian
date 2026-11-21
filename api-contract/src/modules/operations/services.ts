import type { Client, Cancellation } from '../../http/client'
import type { ImageRef, ResourceAction } from '../product-workflows/client'
export type BannerAction = Extract<ResourceAction, { kind: 'none' | 'internal_page' }>
export type TargetAction = { kind: 'mini_program'; app_id: string; path: string } | { kind: 'web_view' | 'copy_link'; url: string } | { kind: 'qr_code'; asset_id: string }
export interface TargetChannel { key: string; label: string; action: TargetAction }
export interface TargetInput {
  school_ids?: string[]; partner_id?: string | null; cooperation_id?: string | null
  revision: number; name: string; provider: string; purpose: string; disclosure: string; contact: string; complaint: string
  authorization: string; qualification: string; channels: TargetChannel[]
}
export interface TargetVersion { version_number: number; content_digest: string; id: string; target_id: string; revision: number; state: 'draft' | 'pending' | 'approved' | 'rejected' | 'withdrawn'; config: TargetInput; reviews: { id: string; actor_id: string; decision: string; reason: string; created_at: string }[] }
export interface ServiceTarget { id: string; revision: number; versions: TargetVersion[] }
export type ShortcutAction = BannerAction | { kind: 'target_channel'; version_id: string; channel: string }
export interface ShortcutInput {
  revision: number; title: string; subtitle: string; badge: string; image_ref: ImageRef; category_id: string | null; cooperation_id: string | null
  action: ShortcutAction; active: boolean; featured: boolean; sort_order: number; starts_at: string | null; ends_at: string | null
}
export interface Shortcut extends ShortcutInput { id: string; school_id: string | null }
export interface CategoryInput { revision: number; name: string; active: boolean; sort_order: number }
export interface PartnerInput { revision: number; name: string; contact: string; active: boolean }
export interface CooperationInput {
  school_ids?: string[]
  revision: number; partner_id: string; name: string; model: 'service' | 'sponsor' | 'cps'; disclosure: string; channel_code: string
  active: boolean; starts_at: string | null; ends_at: string | null
}
export interface MarketingInput { revision: number; name: string; ad_unit_id: string; format: 'custom' | 'rewarded'; reward_scene: 'service_theme' | null; qualification: string; active: boolean }
export interface MarketingSlot { id: string; name: string; ad_unit_id: string; format: 'custom' | 'rewarded'; reward_scene: 'service_theme' | null }
export interface EntityInputs {
  'home-shortcuts': ShortcutInput; 'operation-service-categories': CategoryInput; 'operation-service-targets': TargetInput
  'operation-partners': PartnerInput; 'operation-cooperations': CooperationInput; 'marketing-slots': MarketingInput
}
export type ServiceEntity = keyof EntityInputs
export type EntityRecord<Entity extends ServiceEntity> = Entity extends 'operation-service-targets' ? ServiceTarget : EntityInputs[Entity] & { id: string; school_id?: string | null }
export type ExecutionAction = Exclude<TargetAction, { kind: 'qr_code' }> | { kind: 'qr_code'; image_url: string }
export interface EffectiveChannel { key: string; label: string; action: ExecutionAction }
export interface EffectiveService {
  id: string; school_id: string | null; title: string; subtitle: string; badge: string; image_url: string
  category_id: string | null; category_name: string | null; featured: boolean; sort_order: number
  cooperation_id: string | null
  starts_at: string | null; ends_at: string | null; action: ShortcutAction; channels: EffectiveChannel[]
  disclosure: { provider: string; purpose: string; disclosure: string; contact: string; complaint: string } | null
  cooperation: { name: string; model: string; disclosure: string } | null
  revision: number; override_revision: number; hidden_reason?: string | null; server_time?: string
}
export interface ServiceCatalog {
  id: string; server_time: string; token: string; featured: EffectiveService[]; items: EffectiveService[]
  categories: { id: string; name: string }[]; has_more: boolean; preview: boolean
}
export interface ServiceFilters { version?: string; channel?: string; page?: number; size?: number; search?: string; category_id?: string; at?: string }
export interface ServiceOptions { id: string; targets: { id: string; name: string; provider: string; channels: { key: string; label: string }[] }[]; categories: { id: string; name: string }[]; cooperations: { id: string; name: string; disclosure: string }[] }
export interface ManageSet<Entity extends ServiceEntity> { id: string; token: string; items: EntityRecord<Entity>[] }
export type Presentation = Partial<Pick<ShortcutInput, 'title' | 'subtitle' | 'badge' | 'image_ref' | 'active' | 'featured' | 'sort_order' | 'category_id'>>
export interface OperationEvent { event_id: string; resource_id: string; event_type: 'view' | 'click'; display_cycle: string; occurred_at: string; channel: string | null; target_version_id: string | null; placement: 'home_banner' | 'home_shortcut' | 'more_services' | 'service_detail'; attempt_id: string | null }
export type ActionOutcome = 'api_success' | 'cancel' | 'fail' | 'copy_success' | 'qr_displayed' | 'webview_loaded' | 'webview_failed'
export interface OperationOutcome { event_id: string; click_id: string; outcome: ActionOutcome; occurred_at: string; attempt_id: string; stage: 'navigation_api' | 'clipboard_api' | 'qr_display' | 'webview_load' }
export interface OperationReportRow {
  school_id: string; day: string; resource_id: string; channel: string; target_version: string
  dimension_key: string
  dimensions: { title: string; kind: string; placement: string }; views: number; clicks: number; api_success: number; cancel: number; fail: number
  copy_success: number; qr_displayed: number; unreported: number; ctr: number | null
  webview_loaded: number; webview_failed: number; success_rate: number | null; cancel_rate: number | null; webview_load_rate: number | null
}
export interface ReportFilters { partner_id?: string; school_ids?: string[]; resource_id?: string; target_version_id?: string; cooperation_id?: string; placement?: string; channel?: string }
export interface OperationReport { id: string; report: { pagination: { page: number; size: number; total: number }; coverage: { available_from: string; requested_from: string; partial: boolean; notice: string }; starts_at: string; ends_at: string; timezone: string; notice: string; rows: OperationReportRow[]; summary: { views: number; clicks: number; ctr: number | null }; trends: { day: string; views: number; clicks: number; ctr: number | null }[] } }
export function serviceQuery(filters: ServiceFilters = {}) {
  return Object.entries(filters).filter(([, value]) => value !== undefined && value !== '').map(([key, value], index) => `${index ? '&' : '?'}${encodeURIComponent(key === 'page' ? 'page[number]' : key === 'size' ? 'page[size]' : key)}=${encodeURIComponent(String(value))}`).join('')
}
export function createServiceOperationsClient(client: Client) {
  const root = (school?: string) => school ? `/api/v1/schools/${encodeURIComponent(school)}` : '/api/v1/platform'
  const encoded = encodeURIComponent
  return {
    options: (school?: string) => client.requestResource<ServiceOptions>(`${root(school)}/operation-service-target-options`),
    list: <Entity extends ServiceEntity>(school: string | undefined, entity: Entity, filters: ServiceFilters = {}, signal?: Cancellation) => client.requestResource<ManageSet<Entity>>(`${root(school)}/${entity}${serviceQuery(filters)}`, 'GET', undefined, signal),
    get: <Entity extends ServiceEntity>(school: string | undefined, entity: Entity, id: string) => client.requestResource<EntityRecord<Entity>>(`${root(school)}/${entity}/${encoded(id)}`),
    save: <Entity extends ServiceEntity>(school: string | undefined, entity: Entity, input: EntityInputs[Entity], id?: string) => client.requestResource<EntityRecord<Entity>>(`${root(school)}/${entity}${id ? `/${encoded(id)}` : ''}`, id ? 'PATCH' : 'POST', input),
    remove: (school: string | undefined, entity: ServiceEntity, id: string, revision: number) => client.requestResource<null>(`${root(school)}/${entity}/${encoded(id)}`, 'DELETE', { revision }),
    decision: (target: string, target_revision: number, id: string, revision: number, decision: 'submit' | 'approve' | 'reject' | 'withdraw', reason: string) => client.requestResource<{ id: string; revision: number; state: string }>(`/api/v1/platform/operation-service-targets/${encoded(target)}/${decision}`, 'POST', { revision:target_revision,version_id:id,version_revision:revision,reason }),
    catalog: (school: string, filters: ServiceFilters = {}) => client.requestResource<ServiceCatalog>(`${root(school)}/operation-services${serviceQuery(filters)}`),
    resolve: (school: string, id: string, version?: string, channel?: string) => client.requestResource<EffectiveService>(`${root(school)}/operation-services/${encoded(id)}/resolve${serviceQuery({ version, channel })}`),
    preview: (school: string, filters: ServiceFilters = {}) => client.requestResource<ServiceCatalog>(`${root(school)}/home-shortcuts/preview${serviceQuery(filters)}`),
    draftPreview: (school: string, input: { token: string; at: string; resource_id: string | null; platform_default: boolean; draft: ShortcutInput }) => client.requestResource<ServiceCatalog>(`${root(input.platform_default?undefined:school)}/home-shortcuts/preview`, 'POST', input.platform_default?{...input,school_id:school}:input),
    override: (school: string, id: string, revision: number, token: string, presentation: Presentation) => client.requestResource<{ id: string; token: string }>(`${root(school)}/home-shortcut-overrides/${encoded(id)}`, 'PATCH', { revision, token, presentation }),
    reset: (school: string, id: string, revision: number) => client.requestResource<{ id: string; token: string }>(`${root(school)}/home-shortcut-overrides/${encoded(id)}`, 'DELETE', { revision }),
    batch: (school: string, operation: 'order' | 'batch-status', token: string, items: { id: string; revision: number; active?: boolean }[]) => client.requestResource<{ id: string; token: string }>(`${root(school)}/home-shortcuts/${operation}`, 'POST', { token, items }),
    event: (school: string, event: OperationEvent) => client.requestResource<{ id: string; accepted: boolean }>(`${root(school)}/operation-events`, 'POST', event),
    outcome: (school: string, event: OperationOutcome) => client.requestResource<{ id: string; accepted: boolean }>(`${root(school)}/operation-action-events`, 'POST', event),
    report: (school: string | undefined, starts_at: string, ends_at: string, filters: ReportFilters = {}) => client.requestResource<OperationReport>(`${root(school)}/operation-reports`, 'POST', { starts_at, ends_at, ...filters }),
    readReport: (school: string | undefined, id: string, page = 1, size = 30) => client.requestResource<OperationReport>(`${root(school)}/operation-reports/${encoded(id)}${serviceQuery({page,size})}`),
    exportReport: (school: string | undefined, id: string) => client.requestBinary(`${root(school)}/operation-reports/${encoded(id)}/export`),
    marketing: (school: string) => client.requestResource<{ id: string; items: MarketingSlot[] }>(`${root(school)}/marketing`),
    marketingSettings: (school: string) => client.requestResource<{ id: string; items: { id: string; name: string; format: string; available: boolean; enabled: boolean; revision: number }[] }>(`${root(school)}/marketing-settings`),
    marketingEvent: (school: string, event: { event_id: string; slot_id: string; event_type: 'loaded' | 'error' | 'show_api_success' | 'closed_early' | 'client_completed'; occurred_at: string }) => client.requestResource<{ id: string; accepted: boolean }>(`${root(school)}/marketing-events`, 'POST', event),
    marketingSetting: (school: string, id: string, revision: number, enabled: boolean) => client.requestResource<{ id: string; revision: number; enabled: boolean }>(`${root(school)}/marketing/${encoded(id)}`, 'PATCH', { revision, enabled }),
  }
}
