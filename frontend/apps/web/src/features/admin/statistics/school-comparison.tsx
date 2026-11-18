import { useState } from 'react'
import { useQuery } from '@tanstack/react-query'
import { Alert, Button, Table, Tooltip } from 'antd'
import { statisticsApi, type StatisticsReport } from './api'
import { metricNames, formatValue } from './trends'
export function SchoolComparison({
  report: r,
  userId,
  onSchool,
  exporting,
  onExport,
}: {
  report: StatisticsReport
  userId: string
  onSchool: (id: string) => void
  exporting: boolean
  onExport: () => void
}) {
  const [schoolPage, setSchoolPage] = useState({
    page: 1,
    size: 20,
    sort: 'posts',
    descending: true,
  })
  const schoolQuery = useQuery({
    queryKey: ['linjian', 'statistics', userId, 'school-rows', r.id, schoolPage],
    queryFn: ({ signal }) => statisticsApi.schools(r.id, schoolPage, signal),
    retry: false,
    staleTime: 60000,
  })
  return (
    <>
      <p>人数按学校分别去重，各校人数之和可能大于全站人数。</p>
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
      <Table
        rowKey="id"
        size="small"
        loading={schoolQuery.isFetching}
        dataSource={schoolQuery.data?.items ?? []}
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
        }}
        columns={[
          {
            title: '学校',
            key: 'name',
            sorter: true,
            sortOrder:
              schoolPage.sort === 'name' ? (schoolPage.descending ? 'descend' : 'ascend') : null,
            render: (_, row) => (
              <Button type="link" onClick={() => onSchool(row.id)}>
                {row.name}
              </Button>
            ),
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
                    ? '已记录，历史覆盖未确认'
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
