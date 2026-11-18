import { afterEach, expect, it, vi } from 'vitest'
import { cleanup, render, screen, waitFor } from '@testing-library/react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { MemoryRouter } from 'react-router-dom'
import { StatisticsPage } from './statistics-page'
import { emptyReport } from './charts/test-fixtures'
import { statisticsApi, type StatisticsReport, type StatisticsView } from './api'
vi.mock('./api', () => ({ statisticsApi: { report: vi.fn(), export: vi.fn(), schools: vi.fn() } }))
afterEach(() => {
  cleanup()
  vi.clearAllMocks()
})
const schools = [
  { id: 'a', name: '甲校', status: 'normal' },
  { id: 'b', name: '乙校', status: 'normal' },
]
function show(
  platform: boolean,
  search = '',
  view: StatisticsView = 'overview',
  schoolOptions = schools,
) {
  const client = new QueryClient({ defaultOptions: { queries: { retry: false } } })
  render(
    <QueryClientProvider client={client}>
      <MemoryRouter initialEntries={[`/admin?view=statistics${search}`]}>
        <StatisticsPage
          view={view}
          user={{
            id: 'u',
            platform_admin: platform,
            grants: platform ? [] : [{ school_id: 'a', role: 'admin' }],
          }}
          schools={schoolOptions}
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
      expect.objectContaining({ period: 'month', view: 'overview', comparison: 'none' }),
      expect.any(AbortSignal),
    ),
  )
  expect(screen.getByLabelText('统计学校范围')).toBeTruthy()
  expect(screen.getByRole('heading', { name: '统计概览' })).toBeTruthy()
  expect(statisticsApi.schools).not.toHaveBeenCalled()
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
it('allows a platform administrator to open a school before selector options load', async () => {
  vi.mocked(statisticsApi.report).mockImplementation(() => new Promise<StatisticsReport>(() => {}))
  show(true, '&statistics_school=b', 'overview', [])
  await waitFor(() =>
    expect(statisticsApi.report).toHaveBeenCalledWith(
      'b',
      expect.objectContaining({ view: 'overview' }),
      expect.any(AbortSignal),
    ),
  )
  expect(screen.queryByText('无权查看该学校统计，请选择有管理授权的学校。')).toBeNull()
  expect(screen.getByLabelText('统计学校范围')).toBeTruthy()
})
it('keeps platform scope controls available when a direct-linked school is absent from the list', async () => {
  vi.mocked(statisticsApi.report).mockRejectedValue(new Error('学校不存在'))
  show(true, '&statistics_school=outside-list')
  expect(await screen.findByText('学校不存在')).toBeTruthy()
  expect(statisticsApi.report).toHaveBeenCalledWith(
    'outside-list',
    expect.anything(),
    expect.any(AbortSignal),
  )
  expect(screen.queryByText('无权查看该学校统计，请选择有管理授权的学校。')).toBeNull()
  expect(screen.getByLabelText('统计学校范围')).toBeTruthy()
})
it('does not request another school through a forged statistics URL', () => {
  show(false, '&statistics_school=b')
  expect(statisticsApi.report).not.toHaveBeenCalled()
  expect(screen.getByText('无权查看该学校统计，请选择有管理授权的学校。')).toBeTruthy()
})

it('charts overview summaries without fetching details or school rows', async () => {
  vi.mocked(statisticsApi.report).mockResolvedValue(emptyReport())
  show(true)
  expect(await screen.findByRole('figure', { name: '本期内容量' })).toBeTruthy()
  expect(screen.getByRole('figure', { name: '新增注册与访问活跃' })).toBeTruthy()
  expect(screen.queryByRole('img', { name: /趋势/ })).toBeNull()
  expect(screen.queryByText('查看周期明细')).toBeNull()
  expect(screen.queryByText('导出当前周期 CSV')).toBeNull()
  expect(statisticsApi.schools).not.toHaveBeenCalled()
})
it('renders a known-zero content trend and still reports expired exports', async () => {
  vi.mocked(statisticsApi.report).mockResolvedValue({ ...emptyReport(), view: 'content' })
  vi.mocked(statisticsApi.export).mockRejectedValue(new Error('快照不可用，请刷新后重新导出'))
  show(true, '', 'content')
  expect(await screen.findByRole('figure', { name: '内容统计 · 内容量趋势' })).toBeTruthy()
  expect(screen.getByText('所选期间已记录值均为 0；历史覆盖以数据说明为准。')).toBeTruthy()
  const { fireEvent } = await import('@testing-library/react')
  fireEvent.click(screen.getByRole('button', { name: '导出当前周期 CSV' }))
  expect(await screen.findByText('快照不可用，请刷新后重新导出')).toBeTruthy()
  expect(statisticsApi.export).toHaveBeenCalledWith(undefined, 'snapshot', 'periods')
})

it('loads only a page of school rows from the snapshot and requests another page on navigation', async () => {
  const { fireEvent } = await import('@testing-library/react')
  vi.mocked(statisticsApi.report).mockResolvedValue({
    ...emptyReport(),
    view: 'schools',
    school_count: 41,
  })
  vi.mocked(statisticsApi.schools).mockResolvedValue({
    items: [
      {
        id: 'a',
        name: '甲校',
        status: 'normal',
        metrics: emptyReport().summary,
        coverage: emptyReport().coverage,
        changes: {},
      },
    ],
    page: { number: 1, size: 20, has_more: true },
  })
  show(true, '', 'schools')
  await waitFor(() =>
    expect(statisticsApi.schools).toHaveBeenCalledWith(
      'snapshot',
      { page: 1, size: 20, sort: 'posts', descending: true },
      expect.any(AbortSignal),
    ),
  )
  fireEvent.click(await screen.findByTitle(/Next Page|下一页/))
  await waitFor(() =>
    expect(statisticsApi.schools).toHaveBeenLastCalledWith(
      'snapshot',
      expect.objectContaining({ page: 2, size: 20 }),
      expect.any(AbortSignal),
    ),
  )
})
it('requests a comparison only after it is selected', async () => {
  const { fireEvent } = await import('@testing-library/react')
  vi.mocked(statisticsApi.report).mockResolvedValue({ ...emptyReport(), view: 'content' })
  show(true, '', 'content')
  await screen.findByRole('figure', { name: '内容统计 · 内容量趋势' })
  expect(statisticsApi.report).toHaveBeenLastCalledWith(
    undefined,
    expect.objectContaining({ view: 'content', comparison: 'none' }),
    expect.any(AbortSignal),
  )
  fireEvent.mouseDown(screen.getByLabelText('图表对比期'))
  fireEvent.click(await screen.findByText('同比'))
  await waitFor(() =>
    expect(statisticsApi.report).toHaveBeenLastCalledWith(
      undefined,
      expect.objectContaining({ view: 'content', comparison: 'year' }),
      expect.any(AbortSignal),
    ),
  )
})

it('switches registration modes without requests and preserves mode when refreshing the snapshot', async () => {
  const { fireEvent } = await import('@testing-library/react')
  const report = { ...emptyReport(), view: 'registration' as const }
  vi.mocked(statisticsApi.report).mockResolvedValue(report)
  show(true, '', 'registration')
  await screen.findByRole('figure', { name: '注册统计 · 新增注册趋势' })
  fireEvent.click(screen.getByLabelText('累计注册（截至桶末）'))
  fireEvent.click(screen.getByLabelText('新注册参与人数'))
  fireEvent.click(screen.getByLabelText('新注册参与率'))
  expect(statisticsApi.report).toHaveBeenCalledTimes(1)
  vi.mocked(statisticsApi.report).mockResolvedValue({
    ...report,
    id: 'fresh-snapshot',
    as_of: '2026-11-17T16:00:00Z',
  })
  fireEvent.click(screen.getByRole('button', { name: /刷\s*新/ }))
  await waitFor(() => expect(statisticsApi.report).toHaveBeenCalledTimes(2))
  await screen.findByText(/上海时间；数据截至 2026\/11\/18 00:00/)
  expect(screen.getByRole('figure', { name: '注册统计 · 新注册参与率趋势' })).toBeTruthy()
  expect(statisticsApi.schools).not.toHaveBeenCalled()
})

it('removes cached charts when a refresh loses access to the school', async () => {
  const { fireEvent } = await import('@testing-library/react')
  vi.mocked(statisticsApi.report).mockResolvedValue({ ...emptyReport(), view: 'content' })
  show(false, '', 'content')
  await screen.findByRole('figure', { name: '内容统计 · 内容量趋势' })
  vi.mocked(statisticsApi.report).mockRejectedValue(new Error('该学校管理授权已撤销'))
  fireEvent.click(screen.getByRole('button', { name: /刷\s*新/ }))
  await screen.findByText('该学校管理授权已撤销')
  expect(screen.queryByRole('figure')).toBeNull()
})
