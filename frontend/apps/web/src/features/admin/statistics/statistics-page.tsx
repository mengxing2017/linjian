import { useRef, useState } from 'react'
import { useQuery } from '@tanstack/react-query'
import { useSearchParams } from 'react-router-dom'
import {
  Alert,
  Button,
  Card,
  Col,
  Empty,
  Radio,
  Row,
  Select,
  Skeleton,
  Space,
  Statistic,
  Table,
  Tag,
} from 'antd'
import { statisticsApi, type Metrics, type StatisticsQuery, type Period, type Change } from './api'
import { Trends, metricNames, formatValue, shanghai } from './trends'
interface Props {
  user: { id: string; platform_admin: boolean; grants: { school_id: string; role: string }[] }
  schools: { id: string; name: string; status: string }[]
}
function date(value = new Date()) {
  const parts = new Intl.DateTimeFormat('en-CA', {
    timeZone: 'Asia/Shanghai',
    year: 'numeric',
    month: '2-digit',
    day: '2-digit',
  }).formatToParts(value)
  return ['year', 'month', 'day'].map((key) => parts.find((p) => p.type === key)?.value).join('-')
}
function changeLabel(c?: Change) {
  if (!c || c.state === 'unavailable') return '不可比'
  return `${c.state === 'new' ? '新增' : c.state === 'unchanged' ? '持平' : `${c.percent}%`}（${(c.delta ?? 0) > 0 ? '+' : ''}${c.delta ?? 0}）`
}
export function StatisticsPage({ user, schools }: Props) {
  const [params, setParams] = useSearchParams()
  const authorized = schools.filter(
    (s) =>
      user.platform_admin ||
      (s.status === 'normal' &&
        user.grants.some((g) => g.school_id === s.id && ['admin', 'operator'].includes(g.role))),
  )
  const scope =
    params.get('statistics_school') ?? (user.platform_admin ? '' : (authorized[0]?.id ?? ''))
  const allowed = scope ? authorized.some((s) => s.id === scope) : user.platform_admin
  const period = (params.get('statistics_period') ?? 'month') as Period
  const query: StatisticsQuery =
    period === 'custom'
      ? {
          period,
          start_date: params.get('statistics_start') ?? date(new Date(Date.now() - 29 * 86400000)),
          end_date: params.get('statistics_end') ?? date(),
          granularity: (params.get('statistics_granularity') ??
            'day') as StatisticsQuery['granularity'],
        }
      : { period, anchor_date: params.get('statistics_date') ?? date() }
  query.comparison = 'both'
  if (!scope && user.platform_admin && params.get('statistics_status'))
    query.school_status = params.get('statistics_status') as StatisticsQuery['school_status']
  const q = useQuery({
    queryKey: ['linjian', 'statistics', user.id, scope, query],
    enabled: allowed,
    queryFn: ({ signal }) => statisticsApi.report(scope || undefined, query, signal),
    staleTime: 60000,
    retry: false,
  })
  const [exportError, setExportError] = useState('')
  const [exporting, setExporting] = useState(false)
  const [registration, setRegistration] = useState<'registrations' | 'cumulative_registrations'>(
    'registrations',
  )
  const [comparison, setComparison] = useState('previous')
  const update = (values: Record<string, string>) => {
    setExportError('')
    setParams((previous) => {
      const next = new URLSearchParams(previous)
      for (const [key, value] of Object.entries(values)) next.set(`statistics_${key}`, value)
      return next
    })
  }
  const r = q.data
  const snapshotRef = useRef<string | undefined>(undefined)
  snapshotRef.current = r?.id
  const previous = r?.comparisons.find((c) => c.kind === comparison)
  const download = async (kind: 'periods' | 'schools') => {
    if (!r) return
    setExportError('')
    setExporting(true)
    try {
      const bytes = await statisticsApi.export(scope || undefined, r.id, kind)
      if (snapshotRef.current !== r.id) return
      const url = URL.createObjectURL(new Blob([bytes], { type: 'text/csv;charset=utf-8' }))
      const a = document.createElement('a')
      a.href = url
      a.download = `linjian-statistics-${kind}-${query.anchor_date ?? query.start_date}.csv`
      a.click()
      URL.revokeObjectURL(url)
    } catch (error) {
      setExportError(error instanceof Error ? error.message : '导出失败')
    } finally {
      setExporting(false)
    }
  }
  const drill = (value: string) => {
    if (period === 'day') return
    update({ period: period === 'year' ? 'month' : 'day', date: date(new Date(value)) })
  }
  const shiftedDate = (amount: number) => {
    const d = new Date(`${query.anchor_date ?? date()}T00:00:00Z`)
    if (period === 'month') {
      d.setUTCDate(1)
      d.setUTCMonth(d.getUTCMonth() + amount)
    } else if (period === 'year') {
      d.setUTCMonth(0, 1)
      d.setUTCFullYear(d.getUTCFullYear() + amount)
    } else d.setUTCDate(d.getUTCDate() + amount * (period === 'week' ? 7 : 1))
    return d.toISOString().slice(0, 10)
  }
  const shift = (amount: number) => update({ date: shiftedDate(amount) })
  const labels: Record<string, string> = {
    pending: '待审',
    approved: '通过',
    rejected: '拒绝',
    visible: '可见',
    hidden: '隐藏',
    offline: '下架',
    deleted: '删除',
    future: '尚未发生',
    in_progress: '进行中',
    partial: '部分周期',
    normal: '正常',
  }
  const metricKeys = Object.keys(metricNames) as (keyof Metrics)[]
  if (!allowed) return <Alert type="warning" title="无权查看该学校统计，请选择有管理授权的学校。" />
  return (
    <div className="business-panel" aria-label="业务统计">
      <h2>业务统计</h2>
      <Space wrap style={{ marginBottom: 16 }}>
        {user.platform_admin || authorized.length > 1 ? (
          <Select
            aria-label="统计学校范围"
            style={{ minWidth: 200 }}
            showSearch
            optionFilterProp="label"
            value={scope}
            options={[
              ...(user.platform_admin ? [{ value: '', label: '全站' }] : []),
              ...authorized.map((s) => ({
                value: s.id,
                label: `${s.name}${s.status === 'hidden' ? '（隐藏）' : ''}`,
              })),
            ]}
            onChange={(value) => update({ school: value })}
          />
        ) : (
          <Tag>统计学校：{authorized[0]?.name}</Tag>
        )}
        {!scope && user.platform_admin && (
          <Select
            aria-label="学校状态筛选"
            value={query.school_status ?? ''}
            options={[
              { value: '', label: '全部学校状态' },
              { value: 'normal', label: '正常学校' },
              { value: 'hidden', label: '隐藏学校' },
            ]}
            onChange={(value) => update({ status: value })}
          />
        )}
        <Tag>Asia/Shanghai</Tag>
        <Button onClick={() => void q.refetch()} loading={q.isFetching}>
          刷新
        </Button>
      </Space>
      <div>
        <Space wrap>
          <Radio.Group
            aria-label="报表周期"
            value={period}
            options={[
              { label: '日', value: 'day' },
              { label: '周', value: 'week' },
              { label: '月', value: 'month' },
              { label: '年', value: 'year' },
              { label: '自定义', value: 'custom' },
            ]}
            onChange={(e) => update({ period: e.target.value, date: date() })}
          />
          {period === 'custom' ? (
            <>
              <label>
                开始日期{' '}
                <input
                  aria-label="开始日期"
                  type="date"
                  max={date()}
                  value={query.start_date}
                  onChange={(e) => update({ start: e.target.value })}
                />
              </label>
              <label>
                结束日期{' '}
                <input
                  aria-label="结束日期"
                  type="date"
                  max={date()}
                  value={query.end_date}
                  onChange={(e) => update({ end: e.target.value })}
                />
              </label>
              <Select
                aria-label="统计粒度"
                value={query.granularity}
                options={['day', 'week', 'month'].map((value, i) => ({
                  value,
                  label: ['按日', '按周', '按月'][i],
                }))}
                onChange={(value) => update({ granularity: value })}
              />
            </>
          ) : (
            <>
              <Button onClick={() => shift(-1)}>上一周期</Button>
              <input
                aria-label="周期日期"
                type="date"
                max={date()}
                value={query.anchor_date}
                onChange={(e) => update({ date: e.target.value })}
              />
              <Button disabled={shiftedDate(1) > date()} onClick={() => shift(1)}>
                下一周期
              </Button>
            </>
          )}
          <Select
            aria-label="图表对比期"
            value={comparison}
            options={[
              { value: 'previous', label: '环比' },
              { value: 'year', label: '同比' },
              { value: 'none', label: '不显示对比' },
            ]}
            onChange={setComparison}
          />
        </Space>
      </div>
      {q.error && (
        <Alert
          type="error"
          showIcon
          title={q.error.message}
          action={<Button onClick={() => void q.refetch()}>重试</Button>}
        />
      )}
      {q.isPending && allowed && <Skeleton active />}
      {exportError && (
        <Alert type="error" title={exportError} closable onClose={() => setExportError('')} />
      )}
      {r && (
        <>
          <p>
            {r.full_site ? '全站' : r.school_names.join('、')} · {shanghai(r.window.start_at)} 至{' '}
            {shanghai(r.window.actual_end_at)} · 数据截至{' '}
            {new Date(r.as_of).toLocaleString('zh-CN', { timeZone: 'Asia/Shanghai' })}{' '}
            {r.window.state === 'in_progress' && <Tag>进行中</Tag>}
          </p>
          {r.coverage.reasons.length > 0 && (
            <Alert
              type="info"
              title="部分指标数据不可用"
              description={r.coverage.reasons.join('；')}
            />
          )}
          <Row gutter={[12, 12]}>
            {(
              [
                'posts',
                'comments',
                'post_authors',
                'comment_authors',
                'participants',
                'registrations',
                'cumulative_registrations',
                'active_users',
                'registered_participants',
                'new_participants',
                'new_participation_rate',
                'participation_rate',
              ] as (keyof Metrics)[]
            ).map((key) => (
              <Col xs={24} sm={12} xl={8} key={key}>
                <Card size="small">
                  <Statistic title={metricNames[key]} value={r.summary[key] ?? '—'} />
                  <div>
                    {r.comparisons.map(
                      (c) =>
                        c.changes[key] && (
                          <div key={c.kind}>
                            {c.kind === 'year' ? '同比' : '环比'} {changeLabel(c.changes[key])}{' '}
                            {c.window
                              ? `${shanghai(c.window.start_at)}—${shanghai(c.window.actual_end_at)}`
                              : c.reason}
                          </div>
                        ),
                    )}
                  </div>
                </Card>
              </Col>
            ))}
          </Row>
          {r.summary.posts === 0 && r.summary.comments === 0 && (
            <Empty description="本期无新增帖子与评论" />
          )}
          <Trends
            title="发帖与评论趋势"
            buckets={r.buckets}
            keys={['posts', 'comments']}
            previous={previous?.buckets}
            onSelect={drill}
          />
          <Radio.Group
            value={registration}
            onChange={(e) => setRegistration(e.target.value)}
            options={[
              { label: '新增注册', value: 'registrations' },
              { label: '累计注册（截至桶末）', value: 'cumulative_registrations' },
            ]}
          />
          <Trends
            title="注册人员趋势"
            buckets={r.buckets}
            keys={[registration]}
            previous={registration === 'registrations' ? previous?.buckets : []}
            onSelect={drill}
          />
          <Trends
            title="人员活跃趋势"
            buckets={r.buckets}
            keys={['participants', 'active_users', 'registered_participants']}
            previous={previous?.buckets}
            onSelect={drill}
          />
          <Alert
            type="info"
            title="口径说明"
            description="注册按注册当时学校归属冻结，后来绑校不移动历史注册；活跃按行为学校归属，全站按账号去重。内容参与包含后台作者，注册人员访问活跃排除管理操作。累计注册包含期初基数；未知覆盖显示 —，未来桶不补零。"
          />
          {r.full_site && (
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
          <h3>本期创建内容的当前状态</h3>
          <Row gutter={12}>
            {(['posts', 'comments'] as const).map((kind) => (
              <Col xs={24} md={12} key={kind}>
                <Card size="small" title={kind === 'posts' ? '帖子当前状态' : '评论当前状态'}>
                  <p>
                    当前审核：
                    {Object.entries(r.current_state[kind].audit)
                      .map(([key, value]) => `${labels[key] ?? key} ${value}`)
                      .join('；')}
                  </p>
                  <p>
                    当前展示：
                    {Object.entries(r.current_state[kind].visibility)
                      .map(([key, value]) => `${labels[key] ?? key} ${value}`)
                      .join('；')}
                  </p>
                  <p>当前公开：{r.current_state[kind].public}</p>
                </Card>
              </Col>
            ))}
          </Row>
          <h3>周期明细</h3>
          <Button loading={exporting} onClick={() => void download('periods')}>
            导出当前周期 CSV
          </Button>
          <Table
            size="small"
            rowKey="key"
            dataSource={r.buckets}
            pagination={false}
            scroll={{ x: 1800 }}
            columns={[
              {
                title: '周期（上海时间）',
                key: 'period',
                fixed: 'left',
                width: 200,
                render: (_, b) => (
                  <Button type="link" onClick={() => drill(b.start_at)}>
                    {shanghai(b.start_at)}—{shanghai(b.end_at)}
                    {b.state !== 'complete' ? ` (${labels[b.state] ?? b.state})` : ''}
                  </Button>
                ),
              },
              ...metricKeys.map((key) => ({
                title: metricNames[key],
                key,
                width: 140,
                render: (_: unknown, b: (typeof r.buckets)[number]) => formatValue(b.metrics[key]),
              })),
              {
                title: '覆盖说明',
                key: 'coverage',
                render: (_, b) => b.coverage.reasons.join('；'),
              },
            ]}
            summary={() => (
              <Table.Summary.Row>
                <Table.Summary.Cell index={0}>合计（人数去重，累计取期末）</Table.Summary.Cell>
                {metricKeys.map((key, i) => (
                  <Table.Summary.Cell key={key} index={i + 1}>
                    {formatValue(r.summary[key])}
                  </Table.Summary.Cell>
                ))}
              </Table.Summary.Row>
            )}
          />
          {user.platform_admin && !scope && (
            <>
              <h3>学校对比</h3>
              <p>内容量可按校相加，参与与活跃人数跨校去重，学校人数之和可能大于全站人数。</p>
              <Button loading={exporting} onClick={() => void download('schools')}>
                导出全部学校 CSV
              </Button>
              <Table
                rowKey="id"
                size="small"
                dataSource={r.schools}
                pagination={{ pageSize: 20 }}
                scroll={{ x: 1200 }}
                columns={[
                  {
                    title: '学校',
                    dataIndex: 'name',
                    sorter: (a, b) => a.name.localeCompare(b.name),
                    render: (name, row) => (
                      <Button type="link" onClick={() => update({ school: row.id })}>
                        {name}
                      </Button>
                    ),
                  },
                  {
                    title: '状态',
                    dataIndex: 'status',
                    render: (value: string) => labels[value] ?? value,
                  },
                  ...(
                    [
                      'posts',
                      'comments',
                      'participants',
                      'registrations',
                      'cumulative_registrations',
                      'active_users',
                    ] as const
                  ).map((key) => ({
                    title: metricNames[key],
                    key,
                    sorter: (a: (typeof r.schools)[number], b: (typeof r.schools)[number]) =>
                      (a.metrics[key] ?? -1) - (b.metrics[key] ?? -1),
                    render: (_: unknown, row: (typeof r.schools)[number]) => (
                      <>
                        {formatValue(row.metrics[key])}
                        <div>{changeLabel(row.changes[key])}</div>
                      </>
                    ),
                  })),
                ]}
              />
            </>
          )}
        </>
      )}
    </div>
  )
}
