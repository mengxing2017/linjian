import { useState } from 'react'
import { Button, Collapse, Space, Table } from 'antd'
import type { StatisticsReport, StatisticsView } from './api'
import { metricNames, formatValue, shanghai } from './metrics'
import { TrendPanel } from './charts/trend-panel'
import {
  ContentStateCharts,
  OverviewCharts,
  RegistrationDistribution,
} from './charts/distributions'
import { labels, metricGroups } from './views'
export function PeriodPanel({
  report: r,
  view,
  comparison,
  onSelect: drill,
  exporting,
  canExport = true,
  onExport,
}: {
  report: StatisticsReport
  view: StatisticsView
  comparison: string
  onSelect: (date: string) => void
  canExport?: boolean
  exporting: boolean
  onExport: () => void
}) {
  const metricKeys = metricGroups[view]
  const [detailOpen, setDetailOpen] = useState(false)
  return (
    <>
      {view === 'overview' ? (
        <>
          <p>选择左侧的内容统计、注册统计或活跃统计查看趋势与明细。</p>
          <OverviewCharts report={r} />
        </>
      ) : (
        view !== 'schools' && (
          <>
            <TrendPanel
              report={r}
              view={view}
              comparison={comparison}
              onSelect={r.granularity === 'hour' ? undefined : drill}
            />
            <Space style={{ margin: '16px 0' }}>
              <Button onClick={() => setDetailOpen(!detailOpen)}>
                {detailOpen ? '收起周期明细' : '查看周期明细'}
              </Button>
              <Button disabled={!canExport} loading={exporting} onClick={onExport}>
                导出当前周期 CSV
              </Button>
            </Space>
            {detailOpen && (
              <Table
                size="small"
                rowKey="key"
                dataSource={r.buckets}
                pagination={{ pageSize: 20, showSizeChanger: false }}
                scroll={{ x: 850 }}
                columns={[
                  {
                    title: '周期（上海时间）',
                    key: 'period',
                    width: 200,
                    render: (_, b) => (
                      <Button
                        type="link"
                        disabled={b.state === 'future' || r.granularity === 'hour'}
                        onClick={() => drill(b.start_at)}
                      >
                        {shanghai(b.start_at)}
                        {b.state !== 'complete' ? `（${labels[b.state] ?? b.state}）` : ''}
                      </Button>
                    ),
                  },
                  ...metricKeys.map((key) => ({
                    title: metricNames[key],
                    key,
                    render: (_: unknown, b: (typeof r.buckets)[number]) =>
                      formatValue(b.metrics[key]),
                  })),
                ]}
              />
            )}
            {view === 'content' && (
              <Collapse
                items={[
                  {
                    key: 'state',
                    label: '查看本期创建内容的当前状态',
                    children: <ContentStateCharts report={r} />,
                  },
                ]}
              />
            )}
            {view === 'registration' && r.full_site && (
              <Collapse
                style={{ marginTop: 16 }}
                items={[
                  {
                    key: 'assignment',
                    label: '查看本期注册归属分布',
                    children: <RegistrationDistribution report={r} />,
                  },
                ]}
              />
            )}
          </>
        )
      )}
    </>
  )
}
