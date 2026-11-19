import { useState } from 'react'
import { Alert, Checkbox, Radio, Space } from 'antd'
import type { Metrics, StatisticsReport, StatisticsView } from '../api'
import { metricNames } from '../metrics'
import { ChartCard } from './chart-card'
import { chartColors, trendModel } from './chart-model'

const modes: Record<string, { value: string; label: string; keys: (keyof Metrics)[] }[]> = {
  content: [
    { value: 'volume', label: '内容量', keys: ['posts', 'comments'] },
    {
      value: 'people',
      label: '参与人数',
      keys: ['post_authors', 'comment_authors', 'participants'],
    },
  ],
  registration: [
    { value: 'new', label: '新增注册', keys: ['registrations'] },
    { value: 'total', label: '累计注册（截至桶末）', keys: ['cumulative_registrations'] },
    { value: 'participants', label: '新注册参与人数', keys: ['new_participants'] },
    { value: 'rate', label: '新注册参与率', keys: ['new_participation_rate'] },
  ],
  activity: [
    {
      value: 'people',
      label: '访问与内容参与人数',
      keys: ['active_users', 'registered_participants'],
    },
    { value: 'rate', label: '内容参与率', keys: ['participation_rate'] },
  ],
}
export function TrendPanel({
  report,
  view,
  comparison,
  onSelect,
}: {
  report: StatisticsReport
  view: StatisticsView
  comparison: string
  onSelect?: (id: string) => void
}) {
  const choices = modes[view] ?? []
  const [mode, setMode] = useState(choices[0]?.value)
  const [bars, setBars] = useState(false)
  const [hidden, setHidden] = useState<string[]>([])
  const choice = choices.find((c) => c.value === mode) ?? choices[0]
  if (!choice) return null
  const model = trendModel(
    report,
    `${choice.label}趋势`,
    choice.keys,
    comparison,
    bars && mode !== 'total',
  )
  model.series = model.series.filter(
    (s) => !hidden.some((key) => s.id === key || s.id.startsWith(`${key}-`)),
  )
  if (!model.series.length) model.emptyMessage = '请选择至少一个指标。'
  const selectedComparison = report.comparisons.find((c) => c.kind === comparison)
  const outOfRange =
    model.percent && model.series.some((s) => s.data.some((v) => v != null && (v < 0 || v > 100)))
  return (
    <ChartCard
      model={model}
      onSelect={onSelect}
      controls={
        <Space orientation="vertical" style={{ width: '100%' }}>
          <Radio.Group
            aria-label="趋势指标"
            value={choice.value}
            options={choices.map(({ value, label }) => ({ value, label }))}
            onChange={(event) => setMode(event.target.value)}
          />
          {mode !== 'total' && (
            <Radio.Group
              aria-label="趋势图形"
              value={bars}
              options={[
                { label: '折线图', value: false },
                { label: '分组柱状图', value: true },
              ]}
              onChange={(event) => setBars(event.target.value)}
            />
          )}
          <Space wrap>
            {choice.keys.map((key, i) => (
              <Checkbox
                key={key}
                checked={!hidden.includes(key)}
                onChange={(e) =>
                  setHidden(e.target.checked ? hidden.filter((k) => k !== key) : [...hidden, key])
                }
              >
                <span style={{ color: chartColors[i % chartColors.length] }}>
                  {metricNames[key]}
                </span>
              </Checkbox>
            ))}
          </Space>
          {mode === 'total' && (
            <span>累计包含期初基数，不是本期新增累加；累计模式不展示同比、环比曲线。</span>
          )}
          {choice.keys.some((key) => key.endsWith('_rate')) && (
            <span>
              {view === 'registration'
                ? '各桶新注册内容参与人数 / 各桶新增注册人数'
                : '各桶注册人员内容参与人数 / 各桶访问活跃人数'}
              ；分母为零或覆盖不足时留空，不平均各桶参与率。
            </span>
          )}
          {comparison !== 'none' &&
            mode !== 'total' &&
            selectedComparison &&
            !selectedComparison.window && (
              <Alert type="info" title={selectedComparison.reason ?? '无可比同期'} />
            )}
          {outOfRange && (
            <Alert type="warning" title="参与率超出 0–100%，图表保留原值，请核查统计口径。" />
          )}
        </Space>
      }
    />
  )
}
