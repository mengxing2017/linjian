import type { StatisticsReport } from '../api'
export function emptyReport(): StatisticsReport {
  const coverage = {
    content: true,
    registration: true,
    registration_assignment: true,
    registration_base: true,
    activity: false,
    content_from: null,
    registration_from: null,
    activity_from: null,
    activity_through: null,
    reasons: ['活跃采集未覆盖'],
  }
  const metrics = {
    miniapp_posts: 0, web_posts: 0, seed_posts: 0, unknown_posts: 0,
    posts: 0,
    comments: 0,
    post_authors: 0,
    comment_authors: 0,
    participants: 0,
    registrations: 0,
    cumulative_registrations: 0,
    new_participants: 0,
    active_users: null,
    registered_participants: null,
    daily_posts: 0,
    daily_comments: 0,
    comment_post_ratio: null,
    new_participation_rate: null,
    participation_rate: null,
  }
  const window = {
    key: 'summary',
    start_at: '2026-11-15T16:00:00Z',
    end_at: '2026-11-16T16:00:00Z',
    actual_end_at: '2026-11-16T16:00:00Z',
    state: 'complete' as const,
  }
  const state = { audit: { approved: 0 }, visibility: { visible: 0 }, public: 0 }
  return {
    view: 'overview',
    school_count: 0,
    id: 'snapshot',
    school_id: null,
    school_ids: [],
    school_names: [],
    full_site: true,
    timezone: 'Asia/Shanghai',
    period: 'month',
    granularity: 'day',
    window,
    as_of: window.end_at,
    expires_at: window.end_at,
    definition_version: 1,
    summary: metrics,
    coverage,
    buckets: [{ ...window, key: 'bucket-0', metrics, coverage }],
    comparisons: [],
    current_state: { posts: state, comments: state },
    registration_groups: {},
    schools: [],
  }
}
