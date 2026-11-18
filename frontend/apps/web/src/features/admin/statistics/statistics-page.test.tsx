import { afterEach, expect, it, vi } from 'vitest'
import { cleanup, render, screen, waitFor } from '@testing-library/react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { MemoryRouter } from 'react-router-dom'
import { StatisticsPage } from './statistics-page'
import { statisticsApi, type StatisticsReport } from './api'
vi.mock('./api', () => ({ statisticsApi: { report: vi.fn(), export: vi.fn() } }))
afterEach(() => {
  cleanup()
  vi.clearAllMocks()
})
const schools = [
  { id: 'a', name: '甲校', status: 'normal' },
  { id: 'b', name: '乙校', status: 'normal' },
]
function show(platform: boolean, search = '') {
  const client = new QueryClient({ defaultOptions: { queries: { retry: false } } })
  render(
    <QueryClientProvider client={client}>
      <MemoryRouter initialEntries={[`/admin?view=statistics${search}`]}>
        <StatisticsPage
          user={{ id: 'u', platform_admin: platform, grants: [{ school_id: 'a', role: 'admin' }] }}
          schools={schools}
        />
      </MemoryRouter>
    </QueryClientProvider>,
  )
}
it('opens platform statistics without choosing the first school', async () => {
  vi.mocked(statisticsApi.report).mockImplementation(() => new Promise<StatisticsReport>(() => {}))
  show(true)
  await waitFor(() =>
    expect(statisticsApi.report).toHaveBeenCalledWith(
      undefined,
      expect.objectContaining({ period: 'month' }),
      expect.any(AbortSignal),
    ),
  )
  expect(screen.getByLabelText('统计学校范围')).toBeTruthy()
})
it('fixes a school administrator to their authorized school', async () => {
  vi.mocked(statisticsApi.report).mockRejectedValue(new Error('历史覆盖未确认'))
  show(false)
  await waitFor(() =>
    expect(statisticsApi.report).toHaveBeenCalledWith(
      'a',
      expect.anything(),
      expect.any(AbortSignal),
    ),
  )
  expect(screen.queryByLabelText('统计学校范围')).toBeNull()
  expect(screen.getByText('统计学校：甲校')).toBeTruthy()
  expect(await screen.findByText('历史覆盖未确认')).toBeTruthy()
})
it('does not request another school through a forged statistics URL', () => {
  show(false, '&statistics_school=b')
  expect(statisticsApi.report).not.toHaveBeenCalled()
  expect(screen.getByText('无权查看该学校统计，请选择有管理授权的学校。')).toBeTruthy()
})

it('renders known zeros separately from unavailable activity and reports expired exports', async () => {
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
  vi.mocked(statisticsApi.report).mockResolvedValue({
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
  })
  vi.mocked(statisticsApi.export).mockRejectedValue(new Error('快照不可用，请刷新后重新导出'))
  show(true)
  expect(await screen.findByText('本期无新增帖子与评论')).toBeTruthy()
  expect(screen.getAllByText('—').length).toBeGreaterThan(0)
  expect(screen.getByRole('img', { name: /发帖与评论趋势/ })).toBeTruthy()
  const { fireEvent } = await import('@testing-library/react')
  fireEvent.click(screen.getByRole('button', { name: '导出当前周期 CSV' }))
  expect(await screen.findByText('快照不可用，请刷新后重新导出')).toBeTruthy()
  expect(statisticsApi.export).toHaveBeenCalledWith(undefined, 'snapshot', 'periods')
})
