import { useState } from 'react'
import { useQuery } from '@tanstack/react-query'
import { Alert, Button, Radio, Select, Space, Table, Tooltip } from 'antd'
import { statisticsApi, type StatisticsReport, type Metrics, type StatisticsView } from './api'
import { metricNames, formatValue } from './metrics'
import { ChartCard } from './charts/chart-card'
import { schoolModel } from './charts/chart-model'

const schoolMetrics = [
  'posts',
  'comments',
  'participants',
  'registrations',
  'cumulative_registrations',
  'active_users',
] as const
export function SchoolComparison({
  report: r,
  userId,
  onSchool,
  exporting,
  onExport,
}: {
  report: StatisticsReport
  userId: string
  onSchool: (id: string, view: StatisticsView) => void
  exporting: boolean
  onExport: () => void
}) {
  const [schoolPage, setSchoolPage] = useState({
    page: 1,
    size: 20,
    sort: 'posts',
    descending: true,
  })
  const [metric, setMetric] = useState<keyof Metrics>('posts')
  const openSchool = (id: string) =>
    onSchool(
      id,
      ['registrations', 'cumulative_registrations'].includes(metric)
        ? 'registration'
        : metric === 'active_users'
          ? 'activity'
          : 'content',
    )
  const schoolQuery = useQuery({
    queryKey: ['linjian', 'statistics', userId, 'school-rows', r.id, schoolPage],
    queryFn: ({ signal }) => statisticsApi.schools(r.id, schoolPage, signal),
    retry: false,
    staleTime: 60000,
  })
  const rows = schoolQuery.error ? undefined : schoolQuery.data
  return (
    <>
      <p>人数按学校分别去重，各校人数之和可能大于全站人数。</p>
      <Space wrap>
        <Select
          aria-label="学校对比指标"
          value={metric}
          style={{ minWidth: 200 }}
          options={schoolMetrics.map((key) => ({ value: key, label: metricNames[key] }))}
          onChange={(value) => {
            setMetric(value)
            setSchoolPage({ page: 1, size: 20, sort: value, descending: true })
          }}
        />
        <Radio.Group
          aria-label="学校排名顺序"
          value={schoolPage.descending}
          options={[
            { label: '降序', value: true },
            { label: '升序', value: false },
          ]}
          onChange={(e) =>
            setSchoolPage({ ...schoolPage, page: 1, sort: metric, descending: e.target.value })
          }
        />
      </Space>
      <Button loading={exporting} onClick={onExport}>
        导出全部学校 CSV
      </Button>
      {schoolQuery.error && (
        <Alert
          type="error"
          title={schoolQuery.error.message}
          action={<Button onClick={() => void schoolQuery.refetch()}>重试</Button>}
        />
      )}
      {schoolQuery.isPending && <p role="status">正在加载当前页学校对比……</p>}
      {rows && (
        <ChartCard
          model={schoolModel(
            rows.items,
            metric,
            schoolPage.page,
            r.school_count,
            schoolPage.descending,
            schoolPage.sort,
          )}
          onSelect={openSchool}
        />
      )}
      <Table
        rowKey="id"
        size="small"
        loading={schoolQuery.isFetching}
        dataSource={rows?.items ?? []}
        locale={{ emptyText: '当前筛选下暂无学校' }}
        pagination={{
          current: schoolPage.page,
          pageSize: schoolPage.size,
          total: r.school_count,
          showSizeChanger: false,
        }}
        scroll={{ x: 1000 }}
        onChange={(pagination, _filters, sorter) => {
          const sort = Array.isArray(sorter) ? sorter[0] : sorter
          setSchoolPage({
            page: pagination.current ?? 1,
            size: 20,
            sort: String(sort.columnKey ?? 'posts'),
            descending: sort.order !== 'ascend',
          })
          if (schoolMetrics.includes(sort.columnKey as (typeof schoolMetrics)[number]))
            setMetric(sort.columnKey as keyof Metrics)
        }}
        columns={[
          {
            title: '学校',
            key: 'name',
            sorter: true,
            sortOrder:
              schoolPage.sort === 'name' ? (schoolPage.descending ? 'descend' : 'ascend') : null,
            render: (_, row) => (
              <Button type="link" onClick={() => openSchool(row.id)}>
                {row.name}
              </Button>
            ),
          },
          ...schoolMetrics.map((key) => ({
            title: metricNames[key],
            key,
            sorter: true,
            sortOrder:
              schoolPage.sort === key
                ? schoolPage.descending
                  ? ('descend' as const)
                  : ('ascend' as const)
                : null,
            render: (_: unknown, row: NonNullable<typeof schoolQuery.data>['items'][number]) => (
              <Tooltip
                title={
                  (['posts', 'comments', 'participants'].includes(key) && !row.coverage.content) ||
                  (key === 'registrations' && !row.coverage.registration)
                    ? '数据存在缺口，仅展示现有记录'
                    : row.metrics[key] == null
                      ? '所选期间尚未具备完整数据'
                      : undefined
                }
              >
                <span>{formatValue(row.metrics[key])}</span>
              </Tooltip>
            ),
          })),
        ]}
      />
    </>
  )
}
