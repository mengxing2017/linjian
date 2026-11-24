import type { Bucket, Metrics, SchoolRow, StatisticsReport } from '../api'
import { formatValue, metricNames, metricNote, metricUnit, shanghai } from '../metrics'
import { labels } from '../views'

export const chartColors = ['#1668dc', '#d46b08', '#389e0d']
export interface ChartSeries {
  id: string
  name: string
  unit: string
  color: string
  dashed?: boolean
  data: (number | null)[]
  notes: string[]
}
export interface ChartModel {
  title: string
  categories: string[]
  categoryIds: string[]
  series: ChartSeries[]
  horizontal?: boolean
  percent?: boolean
  bars?: boolean
  zoom?: boolean
  height?: number
  description: string
  emptyMessage?: string
}

function bucketLabel(bucket: Bucket) {
  const start = shanghai(bucket.start_at)
  return bucket.end_at === bucket.actual_end_at && bucket.state !== 'in_progress'
    ? start
    : `${start}（${labels[bucket.state] ?? bucket.state}）`
}
function bucketNote(bucket: Bucket | undefined, key: keyof Metrics) {
  if (!bucket) return '该时段没有对应数据'
  return [
    `${shanghai(bucket.start_at)} 至 ${shanghai(bucket.state === 'future' ? bucket.end_at : bucket.actual_end_at)}`,
    labels[bucket.state] ?? bucket.state,
    bucket.state === 'future' ? '尚未发生' : metricNote(key, bucket.coverage, bucket.metrics[key]),
  ]
    .filter(Boolean)
    .join('；')
}

export function trendModel(
  report: StatisticsReport,
  title: string,
  keys: (keyof Metrics)[],
  comparison: string,
  bars: boolean,
): ChartModel {
  const previous = keys.includes('cumulative_registrations')
    ? undefined
    : report.comparisons.find((c) => c.kind === comparison)
  // Relative buckets are supplied by the server. Preserve longer comparison tails,
  // rather than silently truncating a 31-day comparison to a 30-day current month.
  const length = Math.max(report.buckets.length, previous?.buckets.length ?? 0)
  const series: ChartSeries[] = []
  keys.forEach((key, i) => {
    const data = (rows: Bucket[]) =>
      Array.from({ length }, (_, index) => {
        const bucket = rows[index]
        return !bucket || bucket.state === 'future' ? null : bucket.metrics[key]
      })
    const current: ChartSeries = {
      id: key,
      name: metricNames[key],
      unit: metricUnit(key),
      color: chartColors[i % chartColors.length],
      data: data(report.buckets),
      notes: Array.from({ length }, (_, index) => bucketNote(report.buckets[index], key)),
    }
    series.push(current)
    if (previous?.window && previous.buckets.length) {
      series.push({
        ...current,
        id: `${key}-${comparison}`,
        name: `${metricNames[key]}（${comparison === 'year' ? '同比' : '环比'}）`,
        dashed: true,
        data: data(previous.buckets),
        notes: Array.from({ length }, (_, index) => bucketNote(previous.buckets[index], key)),
      })
    }
  })
  return {
    title,
    categories: Array.from({ length }, (_, index) =>
      report.buckets[index] ? bucketLabel(report.buckets[index]) : `对比期 ${bucketLabel(previous!.buckets[index])}`,
    ),
    categoryIds: Array.from({ length }, (_, index) =>
      report.buckets[index]?.state === 'future' ? '' : (report.buckets[index]?.start_at ?? ''),
    ),
    series,
    bars,
    percent: keys.some((key) => key.endsWith('_rate')),
    zoom: length > 60,
    description: `北京时间；数据截至 ${shanghai(report.as_of)}。${keys.map((key) => `${metricNames[key]}单位：${metricUnit(key)}`).join('；')}。虚线表示所选的同比或环比数据。没有数据的时段不连线，尚未发生的时段不显示数值。${length > 60 ? '默认显示最近 60 个时段，可拖动下方滑块查看其他时段。' : ''}`,
    emptyMessage: length
      ? '所选指标暂无可用数据，请查看数据完整性与统计规则说明。'
      : '所选期间暂无可展示的数据。',
  }
}

export interface CategoryRow {
  id: string
  name: string
  value: number | null
  note?: string
}
export function categoryModel(
  title: string,
  rows: CategoryRow[],
  unit: string,
  description: string,
): ChartModel {
  return {
    title,
    categories: rows.map((r) => r.name),
    categoryIds: rows.map((r) => r.id),
    series: [
      {
        id: 'value',
        name: title,
        unit,
        color: chartColors[0],
        data: rows.map((r) => r.value),
        notes: rows.map((r) =>
          [r.note, r.value == null ? '暂无可用数据' : ''].filter(Boolean).join('；'),
        ),
      },
    ],
    horizontal: true,
    bars: true,
    height: Math.max(260, rows.length * 32 + 64),
    description,
    emptyMessage: rows.length ? '所选指标暂无可用数据。' : '当前范围没有可展示的分类数据。',
  }
}

export function schoolModel(
  rows: SchoolRow[],
  key: keyof Metrics,
  page: number,
  total: number,
  descending: boolean,
  sort: string = key,
): ChartModel {
  return categoryModel(
    `当前页学校${metricNames[key]}对比`,
    rows.map((row, index) => ({
      id: row.id,
      name: `${(page - 1) * 20 + index + 1}. ${row.name}`,
      value: row.metrics[key],
      note: metricNote(key, row.coverage, row.metrics[key]),
    })),
    metricUnit(key),
    `共 ${total} 所学校，当前第 ${page} 页，按${sort === 'name' ? '学校名称' : metricNames[key]}${descending ? '降序' : '升序'}排列。${page === 1 && descending && sort === key ? '本页为该指标前 20 校（不足 20 校时显示实际学校）。' : '本图仅比较当前页，不代表全站前 20 校。'}各校人数不可相加为全站人数；点击学校可查看报表。`,
  )
}

export function pointText(series: ChartSeries, index: number) {
  const value = series.data[index]
  return value == null ? '—（暂无可用数据）' : `${formatValue(value)}${series.unit}`
}
