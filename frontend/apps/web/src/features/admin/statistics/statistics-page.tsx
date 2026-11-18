import { useRef, useState } from 'react'
import { useQuery } from '@tanstack/react-query'
import { useSearchParams } from 'react-router-dom'
import {
  Alert,
  Collapse,
  Button,
  Card,
  Col,
  Radio,
  Row,
  Select,
  Skeleton,
  Space,
  Statistic,
  Tag,
} from 'antd'
import {
  statisticsApi,
  type StatisticsQuery,
  type Period,
  type Change,
  type StatisticsView,
} from './api'
import { metricNames, shanghai } from './metrics'
import { titles, metricGroups, coverageReasons } from './views'
import { PeriodPanel } from './period-panel'
import { SchoolComparison } from './school-comparison'
interface Props {
  view?: StatisticsView
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
export function StatisticsPage({ user, schools, view = 'overview' }: Props) {
  const [params, setParams] = useSearchParams()
  const [comparison, setComparison] = useState('none')
  const metricKeys = metricGroups[view]

  const authorized = schools.filter(
    (s) =>
      user.platform_admin ||
      (s.status === 'normal' &&
        user.grants.some((g) => g.school_id === s.id && ['admin', 'operator'].includes(g.role))),
  )
  const scope =
    view === 'schools'
      ? ''
      : (params.get('statistics_school') ?? (user.platform_admin ? '' : (authorized[0]?.id ?? '')))
  // The school list supplies selector options, not platform administrators' permissions.
  // It may still be loading or may omit a school opened through a direct link.
  const allowed = user.platform_admin || (!!scope && authorized.some((s) => s.id === scope))
  const selectedPeriod = params.get('statistics_period') ?? 'month'
  const period: Period = ['day', 'week', 'month', 'year', 'custom'].includes(selectedPeriod)
    ? (selectedPeriod as Period)
    : 'month'
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
  query.view = view
  query.comparison = comparison as StatisticsQuery['comparison']
  const invalidDates =
    period === 'custom'
      ? !/^\d{4}-\d{2}-\d{2}$/.test(query.start_date ?? '') ||
        !/^\d{4}-\d{2}-\d{2}$/.test(query.end_date ?? '') ||
        query.start_date! > query.end_date! ||
        query.end_date! > date() ||
        (Date.parse(query.end_date!) - Date.parse(query.start_date!)) / 86400000 >= 366
      : !/^\d{4}-\d{2}-\d{2}$/.test(query.anchor_date ?? '') || query.anchor_date! > date()

  if (!scope && user.platform_admin && params.get('statistics_status'))
    query.school_status = params.get('statistics_status') as StatisticsQuery['school_status']
  const q = useQuery({
    queryKey: ['linjian', 'statistics', user.id, scope, query],
    enabled: allowed && !invalidDates,
    queryFn: ({ signal }) => statisticsApi.report(scope || undefined, query, signal),
    staleTime: 60000,
    retry: false,
  })
  const [exportError, setExportError] = useState('')
  const [exporting, setExporting] = useState(false)
  const update = (values: Record<string, string>) => {
    setExportError('')
    setParams((previous) => {
      const next = new URLSearchParams(previous)
      for (const [key, value] of Object.entries(values)) next.set(`statistics_${key}`, value)
      return next
    })
  }
  const r = invalidDates || q.error || !allowed ? undefined : q.data
  const snapshotRef = useRef<string | undefined>(undefined)
  snapshotRef.current = r?.id
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
    if (Number.isNaN(d.getTime())) return date()
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
  if (!allowed)
    return (
      <Alert
        type="warning"
        title="无权查看该学校统计，请选择有管理授权的学校。"
        action={
          authorized.length > 0 ? (
            <Button onClick={() => update({ school: authorized[0].id })}>返回已授权学校</Button>
          ) : undefined
        }
      />
    )
  return (
    <div className="business-panel" aria-label={titles[view]}>
      <h2>{titles[view]}</h2>
      <Space wrap style={{ marginBottom: 16 }}>
        {view === 'schools' ? (
          <Tag>全部学校</Tag>
        ) : user.platform_admin || authorized.length > 1 ? (
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
          {view !== 'overview' && view !== 'schools' && (
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
          )}
        </Space>
      </div>
      {invalidDates && (
        <Alert type="warning" title="请选择有效日期，自定义范围最多 366 天且不能包含未来日期。" />
      )}
      {q.error && (
        <Alert
          type="error"
          showIcon
          title={q.error.message}
          action={<Button onClick={() => void q.refetch()}>重试</Button>}
        />
      )}
      {q.isPending && allowed && !invalidDates && !q.error && <Skeleton active />}
      {exportError && (
        <Alert type="error" title={exportError} closable onClose={() => setExportError('')} />
      )}
      {r && (
        <>
          <p>
            {r.full_site ? '全站' : r.school_names.join('、') || '全站（已筛选）'} ·{' '}
            {shanghai(r.window.start_at)} 至 {shanghai(r.window.actual_end_at)} · 上海时间
            {r.window.state === 'in_progress' && <Tag>进行中</Tag>}
          </p>
          <Row gutter={[12, 12]}>
            {metricKeys.map((key) => (
              <Col xs={24} sm={12} xl={view === 'overview' ? 6 : 8} key={key}>
                <Card size="small">
                  <Statistic title={metricNames[key]} value={r.summary[key] ?? '—'} />
                  {r.summary[key] == null ? (
                    <span>暂未具备完整数据</span>
                  ) : (
                    (([
                      'posts',
                      'comments',
                      'post_authors',
                      'comment_authors',
                      'participants',
                    ].includes(key) &&
                      !r.coverage.content) ||
                      (key === 'registrations' && !r.coverage.registration)) && (
                      <span>已记录，历史覆盖未确认</span>
                    )
                  )}
                  {r.comparisons.map(
                    (c) =>
                      c.changes[key] && (
                        <div key={c.kind}>
                          {c.kind === 'year' ? '同比' : '环比'} {changeLabel(c.changes[key])}
                        </div>
                      ),
                  )}
                </Card>
              </Col>
            ))}
          </Row>
          {view === 'content' && !r.coverage.content && (
            <Alert
              type="info"
              title="以下为所选期间的已记录内容，历史完整性尚未确认，不计算增长率。"
            />
          )}
          {view === 'registration' && !r.coverage.registration && (
            <Alert type="info" title="以下新增注册为已验证时间的记录，历史完整性尚未确认。" />
          )}
          {view === 'activity' && !r.coverage.activity && (
            <Alert type="info" title="所选期间的活跃采集尚未完整覆盖，暂不展示人数和增长率。" />
          )}
          <PeriodPanel
            key={`period-${view}-${user.id}-${scope}`}
            report={r}
            view={view}
            comparison={comparison}
            onSelect={drill}
            exporting={exporting}
            onExport={() => void download('periods')}
          />
          {view === 'schools' && (
            <SchoolComparison
              key={`schools-${r.id}`}
              report={r}
              userId={user.id}
              exporting={exporting}
              onExport={() => void download('schools')}
              onSchool={(id, target) =>
                setParams((p) => {
                  const next = new URLSearchParams(p)
                  next.set('view', `statistics-${target}`)
                  next.set('statistics_school', id)
                  return next
                })
              }
            />
          )}
          <Collapse
            style={{ marginTop: 16 }}
            items={[
              {
                key: 'definitions',
                label: coverageReasons(r, view).length ? '数据覆盖与统计口径说明' : '统计口径说明',
                children: (
                  <>
                    {coverageReasons(r, view).length > 0 && (
                      <p>{coverageReasons(r, view).join('；')}</p>
                    )}
                    <p>
                      注册按注册时学校归属冻结；活跃按行为学校归属。全站人数按账号去重。标有“已记录”的数量只反映现有记录，不代表历史完整总量；—
                      表示指标不可用或不可比，未发生的未来周期留空。
                    </p>
                    <p>数据截至 {shanghai(r.as_of)}</p>
                  </>
                ),
              },
            ]}
          />
        </>
      )}
    </div>
  )
}
