import { afterEach, expect, it, vi } from 'vitest'
import { cleanup, fireEvent, render, screen } from '@testing-library/react'
import { TrendPanel } from './trend-panel'
import { emptyReport } from './test-fixtures'
import { ContentStateCharts, RegistrationDistribution } from './distributions'

vi.mock('./echarts-chart', () => ({ default: () => <div data-testid="chart-renderer" /> }))
afterEach(cleanup)

it('keeps registration choices available when all values are unknown and changes locally', () => {
  const report = emptyReport()
  report.buckets = report.buckets.map((b) => {
    const metrics = { ...b.metrics }
    for (const key of Object.keys(metrics) as (keyof typeof metrics)[]) metrics[key] = null
    return { ...b, metrics }
  })
  render(<TrendPanel report={report} view="registration" comparison="none" />)
  expect(screen.getByText('所选指标暂无可用数据，请查看数据覆盖与统计口径说明。')).toBeTruthy()
  fireEvent.click(screen.getByLabelText('累计注册（截至桶末）'))
  expect(screen.getByRole('figure', { name: '注册统计 · 累计注册（截至桶末）趋势' })).toBeTruthy()
  expect(screen.getByText(/累计包含期初基数/)).toBeTruthy()
  fireEvent.click(screen.getByLabelText('新注册参与率'))
  expect(screen.getByText(/各桶新注册内容参与人数/)).toBeTruthy()
})

it('keeps zero trends visible and explains hiding every series', () => {
  render(<TrendPanel report={emptyReport()} view="content" comparison="none" />)
  expect(screen.getByText(/所选期间可用指标均为 0/)).toBeTruthy()
  fireEvent.click(screen.getByLabelText('新增帖子'))
  fireEvent.click(screen.getByLabelText('新增评论'))
  expect(screen.getByText('请选择至少一个指标。')).toBeTruthy()
  fireEvent.click(screen.getByLabelText('参与人数'))
  expect(screen.getByRole('figure', { name: '内容统计 · 参与人数趋势' })).toBeTruthy()
})

it('keeps mode on snapshot refresh and offers activity rates without mixing count axes', () => {
  const report = emptyReport()
  const { rerender } = render(<TrendPanel report={report} view="activity" comparison="none" />)
  fireEvent.click(screen.getByLabelText('内容参与率'))
  rerender(
    <TrendPanel report={{ ...report, id: 'new-snapshot' }} view="activity" comparison="none" />,
  )
  expect(screen.getByRole('figure', { name: '活跃统计 · 内容参与率趋势' })).toBeTruthy()
  expect(screen.getByText(/各桶注册人员内容参与人数/)).toBeTruthy()
})

it('separates audit and visibility snapshots and retains unknown registration assignment', () => {
  const report = emptyReport()
  report.registration_groups = { known_school: 2, unassigned: 1, unknown: 3 }
  render(
    <>
      <ContentStateCharts report={report} />
      <RegistrationDistribution report={report} />
    </>,
  )
  expect(screen.getByRole('figure', { name: '帖子当前审核分布' })).toBeTruthy()
  expect(screen.getByRole('figure', { name: '帖子当前展示分布' })).toBeTruthy()
  const buttons = screen.getAllByRole('button', { name: '查看图表数据' })
  fireEvent.click(buttons[buttons.length - 1])
  expect(screen.getByText('未归属学校')).toBeTruthy()
  expect(screen.getByText('1人')).toBeTruthy()
  expect(screen.getByText('学校归属数据缺失')).toBeTruthy()
  expect(screen.getByText('3人')).toBeTruthy()
})
