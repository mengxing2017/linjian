import { afterEach, expect, it, vi } from 'vitest'
import { cleanup, fireEvent, render, screen } from '@testing-library/react'
import { ChartCard } from './chart-card'
import { categoryModel } from './chart-model'

const state = vi.hoisted(() => ({ fail: true }))
vi.mock('./echarts-chart', () => ({
  default: () => {
    if (state.fail) throw new Error('chunk failed')
    return <div data-testid="loaded-chart" />
  },
}))
afterEach(() => {
  cleanup()
  vi.restoreAllMocks()
})

it('keeps data accessible on renderer failure and retries loading without a report request', async () => {
  // React logs caught render errors; the user-facing recovery is asserted below.
  vi.spyOn(console, 'error').mockImplementation(() => {})
  render(
    <ChartCard
      model={categoryModel('学校对比', [{ id: 'a', name: '甲校', value: 12 }], '人', '当前页')}
    />,
  )
  await screen.findByText('图表加载失败，仍可查看图表数据或周期明细。')
  fireEvent.click(screen.getByRole('button', { name: '查看图表数据' }))
  expect(screen.getByText('甲校')).toBeTruthy()
  expect(screen.getByText('12人')).toBeTruthy()
  state.fail = false
  fireEvent.click(screen.getByRole('button', { name: '重新加载图表' }))
  expect(await screen.findByTestId('loaded-chart')).toBeTruthy()
})
