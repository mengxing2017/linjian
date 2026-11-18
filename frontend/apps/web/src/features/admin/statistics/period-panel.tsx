import { useState } from 'react'
import { Button, Card, Col, Collapse, Empty, Radio, Row, Space, Table } from 'antd'
import type { StatisticsReport, StatisticsView } from './api'
import { Trends, metricNames, formatValue, shanghai } from './trends'
import { labels, metricGroups } from './views'
export function PeriodPanel({
  report: r,
  view,
  comparison,
  onSelect: drill,
  exporting,
  onExport,
}: {
  report: StatisticsReport
  view: StatisticsView
  comparison: string
  onSelect: (date: string) => void
  exporting: boolean
  onExport: () => void
}) {
  const metricKeys = metricGroups[view]
  const [detailOpen, setDetailOpen] = useState(false)
  const [registration, setRegistration] = useState<'registrations' | 'cumulative_registrations'>(
    'registrations',
  )
  const previous = r.comparisons.find((c) => c.kind === comparison)
  return (
    <>
      {view === 'overview' ? (
        <>
          <p>选择左侧的内容统计、注册统计或活跃统计查看趋势与明细。</p>
          {metricKeys.every((key) => r.summary[key] == null || r.summary[key] === 0) && (
            <Empty description="当前范围暂无可展示的统计数据，可调整学校或日期。" />
          )}
        </>
      ) : (
        view !== 'schools' && (
          <>
            {r.buckets.some((b) =>
              metricKeys.some((key) => b.metrics[key] != null && b.metrics[key] !== 0),
            ) ? (
              <>
                {view === 'registration' && (
                  <Radio.Group
                    value={registration}
                    onChange={(e) => setRegistration(e.target.value)}
                    options={[
                      { label: '新增注册', value: 'registrations' },
                      { label: '累计注册（截至桶末）', value: 'cumulative_registrations' },
                    ]}
                  />
                )}
                <Trends
                  title={
                    view === 'content'
                      ? '发帖与评论趋势'
                      : view === 'registration'
                        ? '注册人员趋势'
                        : '人员活跃趋势'
                  }
                  buckets={r.buckets}
                  keys={
                    view === 'content'
                      ? ['posts', 'comments']
                      : view === 'registration'
                        ? [registration]
                        : ['active_users', 'registered_participants']
                  }
                  previous={
                    registration === 'cumulative_registrations' && view === 'registration'
                      ? []
                      : previous?.buckets
                  }
                  onSelect={drill}
                />
              </>
            ) : (
              <Empty
                description={
                  metricKeys.some((key) => r.summary[key] != null)
                    ? '所选期间暂无记录，可调整学校或日期。'
                    : '所选期间暂未具备完整数据，可调整日期查看其他期间。'
                }
              />
            )}
            <Space style={{ margin: '16px 0' }}>
              <Button onClick={() => setDetailOpen(!detailOpen)}>
                {detailOpen ? '收起周期明细' : '查看周期明细'}
              </Button>
              <Button loading={exporting} onClick={onExport}>
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
                      <Button type="link" onClick={() => drill(b.start_at)}>
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
                    children: (
                      <Row gutter={12}>
                        {(['posts', 'comments'] as const).map((kind) => (
                          <Col xs={24} md={12} key={kind}>
                            <Card
                              size="small"
                              title={kind === 'posts' ? '帖子当前状态' : '评论当前状态'}
                            >
                              <p>
                                当前审核：
                                {Object.entries(r.current_state[kind]?.audit ?? {})
                                  .map(([key, value]) => `${labels[key] ?? key} ${value}`)
                                  .join('；')}
                              </p>
                              <p>
                                当前展示：
                                {Object.entries(r.current_state[kind]?.visibility ?? {})
                                  .map(([key, value]) => `${labels[key] ?? key} ${value}`)
                                  .join('；')}
                              </p>
                              <p>当前公开：{r.current_state[kind]?.public ?? 0}</p>
                            </Card>
                          </Col>
                        ))}
                      </Row>
                    ),
                  },
                ]}
              />
            )}
            {view === 'registration' && r.full_site && (
              <p>
                本期注册归属：
                {Object.entries(r.registration_groups)
                  .map(
                    ([key, value]) =>
                      `${({ known_school: '已知学校', unassigned: '未归属学校', unknown: '历史归属未知' } as Record<string, string>)[key]} ${value}`,
                  )
                  .join('；')}
              </p>
            )}
          </>
        )
      )}
    </>
  )
}
