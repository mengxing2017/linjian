import { expect, it, vi } from 'vitest'
import { emptyReport } from './test-fixtures'
import { categoryModel, schoolModel, trendModel } from './chart-model'
import { chartOption, init } from './echarts-runtime'

it('preserves zero, unknown, future and partially covered observed values separately', () => {
  const report = emptyReport()
  const bucket = report.buckets[0]
  report.buckets = [
    bucket,
    { ...bucket, metrics: { ...bucket.metrics, posts: null } },
    { ...bucket, state: 'future', metrics: { ...bucket.metrics, posts: 999 } },
    {
      ...bucket,
      coverage: { ...bucket.coverage, content: false },
      metrics: { ...bucket.metrics, posts: 2 },
    },
  ]
  const model = trendModel(report, '内容趋势', ['posts'], 'none', false)
  expect(model.series[0].data).toEqual([0, null, null, 2])
  expect(model.categoryIds[2]).toBe('')
  expect(model.series[0].notes[3]).toContain('数据存在缺口，仅展示现有记录')
  const option = chartOption(model)
  expect(option.series).toEqual([
    expect.objectContaining({ connectNulls: false, smooth: false, data: [0, null, null, 2] }),
  ])
})

it('uses server cumulative bases and never adds registrations or comparison totals', () => {
  const report = emptyReport()
  const bucket = report.buckets[0]
  report.buckets = [
    { ...bucket, metrics: { ...bucket.metrics, registrations: 2, cumulative_registrations: 102 } },
    { ...bucket, metrics: { ...bucket.metrics, registrations: 0, cumulative_registrations: null } },
    { ...bucket, metrics: { ...bucket.metrics, registrations: 3, cumulative_registrations: 105 } },
  ]
  report.comparisons = [
    {
      kind: 'year',
      window: bucket,
      reason: null,
      metrics: bucket.metrics,
      changes: {},
      buckets: [bucket],
    },
  ]
  const model = trendModel(report, '累计注册', ['cumulative_registrations'], 'year', false)
  expect(model.series).toHaveLength(1)
  expect(model.series[0].data).toEqual([102, null, 105])
})

it('retains longer comparison tails without inventing current values or drill targets', () => {
  const report = emptyReport()
  const bucket = report.buckets[0]
  report.comparisons = [
    {
      kind: 'previous',
      window: bucket,
      reason: null,
      metrics: bucket.metrics,
      changes: {},
      buckets: [bucket, { ...bucket, metrics: { ...bucket.metrics, posts: 5 } }],
    },
  ]
  const model = trendModel(report, '内容趋势', ['posts'], 'previous', false)
  expect(model.categories).toHaveLength(2)
  expect(model.categories[1]).toContain('对比期 2026/11/16')
  expect(model.series[0].notes[1]).toBe('该时段没有对应数据')
  expect(model.categoryIds[1]).toBe('')
  expect(model.series[0].data).toEqual([0, null])
  expect(model.series[1]).toMatchObject({ dashed: true, data: [0, 5] })
  expect(model.series[1].notes[1]).toContain('2026/11/16')
})

it('preserves missing category labels and distinguishes page rankings from global top schools', () => {
  const report = emptyReport()
  const row = {
    id: 'a',
    name: '甲校',
    status: 'normal',
    metrics: { ...report.summary, posts: null },
    coverage: report.coverage,
    changes: {},
  }
  const model = schoolModel([row], 'posts', 2, 41, true)
  expect(model.categories).toEqual(['21. 甲校'])
  expect(model.series[0].data).toEqual([null])
  expect(model.description).toContain('不代表全站前 20 校')
  const nameOrder = schoolModel([row], 'posts', 1, 41, true, 'name')
  expect(nameOrder.description).toContain('按学校名称')
  expect(nameOrder.description).toContain('不代表全站前 20 校')
})

it('renders real SVG charts for zero counts, missing categories and 400 buckets', () => {
  // SVG SSR can use ECharts' approximate text measurements. Real browser layout
  // is verified separately; JSDOM intentionally has no canvas implementation.
  const canvas = vi.spyOn(HTMLCanvasElement.prototype, 'getContext').mockImplementation(() => null)
  const report = emptyReport()
  report.buckets = Array.from({ length: 400 }, (_, i) => ({
    ...report.buckets[0],
    key: `bucket-${i}`,
    metrics: { ...report.summary, posts: i === 1 ? null : 0 },
  }))
  const trend = trendModel(report, '400 个时段', ['posts'], 'none', false)
  expect(trend.zoom).toBe(true)
  expect(trend.description).toContain('北京时间；数据截至')
  expect(trend.description).toContain('默认显示最近 60 个时段')
  expect(trend.description).not.toMatch(/桶|另发请求|上海时间/)
  const chart = init(null, undefined, { renderer: 'svg', ssr: true, width: 1000, height: 320 })
  try {
    chart.setOption(chartOption(trend))
    expect(chart.renderToSVGString()).toContain('<path')
    const categories = categoryModel(
      '状态分布',
      [
        { id: 'a', name: '已知零值', value: 0 },
        { id: 'b', name: '未知类别', value: null },
      ],
      '人',
      '分类数据',
    )
    chart.setOption(chartOption(categories), { notMerge: true })
    const svg = chart.renderToSVGString()
    expect(svg).toContain('已知零值')
    // Labels may be visually truncated; retain the full missing-data label in
    // the axis model and keep the category visible in the generated SVG.
    expect(svg).toContain('未知类别')
    expect(chart.getOption().yAxis).toEqual(
      expect.arrayContaining([
        expect.objectContaining({ data: ['已知零值', '未知类别（暂无数据）'] }),
      ]),
    )
  } finally {
    chart.dispose()
    canvas.mockRestore()
  }
})

it('explains empty periods without exposing internal grouping terms', () => {
  const report = emptyReport()
  report.buckets = []
  const model = trendModel(report, '内容趋势', ['posts'], 'none', false)
  expect(model.emptyMessage).toBe('所选期间暂无可展示的数据。')
  expect(model.series[0].data).toEqual([])
})
