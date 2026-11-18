import type { Client, Cancellation } from '../../http/client'
export type StatisticsView = 'overview' | 'content' | 'registration' | 'activity' | 'schools'
export type Period = 'day' | 'week' | 'month' | 'year' | 'custom'
export interface StatisticsQuery {
  view?: StatisticsView; period?: Period; anchor_date?: string; start_date?: string; end_date?: string
  granularity?: 'hour' | 'day' | 'week' | 'month'; comparison?: 'none' | 'previous' | 'year' | 'both'
  school_ids?: string; school_status?: 'normal' | 'hidden'
}
export interface Metrics {
  posts: number | null; comments: number | null; post_authors: number | null; comment_authors: number | null
  participants: number | null; registrations: number | null; cumulative_registrations: number | null
  new_participants: number | null; active_users: number | null; registered_participants: number | null
  daily_posts: number | null; daily_comments: number | null
  comment_post_ratio: number | null; new_participation_rate: number | null; participation_rate: number | null
}
export interface Coverage { content_from: string | null; registration_from: string | null; activity_from: string | null; activity_through: string | null; content: boolean; registration: boolean; registration_assignment: boolean; registration_base: boolean; activity: boolean; reasons: string[] }
export interface Window { key: string; start_at: string; end_at: string; actual_end_at: string; state: 'future' | 'in_progress' | 'complete' | 'partial' }
export interface Bucket extends Window { metrics: Metrics; coverage: Coverage }
export interface Change { current: number | null; previous: number | null; delta: number | null; percent: number | null; state: 'new' | 'unchanged' | 'comparable' | 'unavailable' }
export interface Comparison { kind: string; window: Window | null; reason: string | null; metrics: Metrics | null; changes: Record<string, Change>; buckets: Bucket[] }
export interface SchoolRow { id: string; name: string; status: string; metrics: Metrics; coverage: Coverage; changes: Record<string, Change> }
export interface StatisticsReport {
  view: StatisticsView; school_count: number;
  id: string; school_id: string | null; school_ids: string[]; school_names: string[]; full_site: boolean
  timezone: string; period: Period; granularity: string; window: Window; as_of: string; expires_at: string
  definition_version: number; summary: Metrics; coverage: Coverage; buckets: Bucket[]; comparisons: Comparison[]
  current_state: Record<'posts' | 'comments', { audit: Record<string, number>; visibility: Record<string, number>; public: number }>
  registration_groups: Record<string, number>; schools: SchoolRow[]
}
export interface ActivityInput { event_id: string; kind: 'feed_open' | 'post_open'; school_id: string; post_id?: string }
export function createStatisticsClient(client: Client) {
  const root = (school?: string) => school ? `/api/v1/schools/${encodeURIComponent(school)}/statistics` : '/api/v1/platform/statistics'
  const query = (values: Record<string, unknown>) => Object.entries(values).filter(([, v]) => v !== undefined && v !== '').map(([k, v]) => `${encodeURIComponent(k)}=${encodeURIComponent(String(v))}`).join('&')
  return {
    report: (school: string | undefined, values: StatisticsQuery, signal?: Cancellation) => client.requestResource<StatisticsReport>(`${root(school)}?${query({ ...values })}`, 'GET', undefined, signal),
    schools: (snapshot: string, values: { page?: number; size?: number; sort?: string; descending?: boolean } = {}, signal?: Cancellation) => client.requestCollection<SchoolRow>(`${root()}/snapshots/${encodeURIComponent(snapshot)}/schools?${query(values)}`, signal),
    export: (school: string | undefined, snapshot: string, kind: 'periods' | 'schools' = 'periods', signal?: Cancellation) => client.requestBinary(`${root(school)}/snapshots/${encodeURIComponent(snapshot)}/export?kind=${kind}`, signal),
    activity: (input: ActivityInput) => client.requestResource<{ id: string; accepted: boolean }>('/api/v1/miniapp/activity-events', 'POST', input),
  }
}
