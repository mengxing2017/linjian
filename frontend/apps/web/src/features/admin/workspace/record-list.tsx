import { useState, type ReactNode } from 'react'
import { useQuery } from '@tanstack/react-query'
import {
  Alert, Button, Descriptions, Drawer, Input, Space, Table, Tag,
  type TableColumnsType, type DescriptionsProps,
} from 'antd'
import { workspace, type Row } from './api'
import { ListPanel } from '../../../shared/list-panel'
import { PaginationControls } from '../../../shared/pagination-controls'
import { formatDate } from '../../../shared/format-date'
import { label } from '../../../shared/label'
import { stateLabel } from './state-label'

type ListResource = 'comments' | 'cases' | 'resources' | 'audit'
export interface RecordDetailProps<T extends Row = Row> {
  record: T
  close: () => void
  reload: () => Promise<boolean>
}

/** Shared list mechanics; columns, filters and detail actions belong to each business page. */
interface RecordListProps<T extends Row = Row> {
  school: string
  resource: ListResource
  columns: (open: (id: string) => void) => TableColumnsType<T>
  detail: (props: RecordDetailProps<T>) => ReactNode
  extraQuery?: Record<string, string>
  status?: string
  onStatusChange?: (value: string) => void
  filters?: (changeStatus: (value: string) => void) => ReactNode
  actions?: ReactNode
  rowActions?: (record: T, close: () => void) => ReactNode
  paginated?: boolean
  loader?: (school: string, query: string, signal?: AbortSignal) => ReturnType<typeof workspace.list<T>>
  showCreatedAt?: boolean
  searchPlaceholder?: string
  scrollWidth?: number
}

export function RecordList<T extends Row = Row>({
  school, resource, columns, detail, status, onStatusChange, filters, actions, extraQuery,
  rowActions, paginated = true, scrollWidth = 880, loader, showCreatedAt = true, searchPlaceholder = '搜索标题或正文',
}: RecordListProps<T>) {
  const [page, setPage] = useState(1)
  const [search, setSearch] = useState('')
  const [selectedId, setSelectedId] = useState<string>()
  const query = `?${new URLSearchParams({
    'page[number]': String(page), 'page[size]': '30',
    ...(search ? { search } : {}), ...(status ? { status } : {}), ...extraQuery,
  })}`
  const q = useQuery({
    queryKey: ['business', school, resource, query],
    queryFn: ({ signal }) => loader ? loader(school, query, signal) : workspace.list<T>(school, resource, query, signal),
  })
  const close = () => setSelectedId(undefined)
  const selected = q.data?.items.find(record => record.id === selectedId)
  return (
    <>
      {q.error && <Alert type="error" title={q.error.message} />}
      <ListPanel
        filters={<Space wrap>
          <Input.Search aria-label="搜索内容" placeholder={searchPlaceholder} onSearch={value => { setSearch(value); setPage(1); close() }} />
          {filters?.(value => { onStatusChange?.(value); setPage(1); close() })}
          <Button onClick={() => void q.refetch()} loading={q.isFetching}>刷新</Button>
        </Space>}
        actions={actions}
        footer={paginated && <PaginationControls page={page} more={q.data?.page?.has_more} busy={q.isFetching} change={setPage} />}
      >
        <Table<T>
          rowKey="id"
          loading={q.isFetching}
          dataSource={q.data?.items ?? []}
          pagination={false}
          scroll={{ x: scrollWidth }}
          columns={[
            ...columns(setSelectedId),
            ...(showCreatedAt ? [{ title: '创建时间', key: 'created', width: 180, render: (_: unknown, row: T) => formatDate(row.created_at) }] : []),
            { title: '操作', key: 'actions', width: 150, fixed: 'right', render: (_, row) => <Space size={0}>
              <Button type="link" onClick={() => setSelectedId(row.id)}>详情</Button>
              {rowActions?.(row, close)}
            </Space> },
          ]}
        />
      </ListPanel>
      {selected && <Drawer key={selected.id} open size="large" title={label(selected.title ?? selected.name ?? selected.action) || '记录详情'} onClose={close}>
        {detail({ record: selected, close, reload: async () => (await q.refetch()).isSuccess })}
      </Drawer>}
    </>
  )
}

export function recordTitleColumn(open: (id: string) => void): TableColumnsType<Row>[number] {
  return { title: '标题 / 名称', key: 'title', width: 240, render: (_, row) => <Button className="record-title" type="link" onClick={() => open(row.id)}>
    {label(row.post_title ?? row.title ?? row.name ?? row.group_name ?? row.display_name ?? row.action) || row.id}
  </Button> }
}

export function defaultRecordColumns(open: (id: string) => void): TableColumnsType<Row> {
  return [
    recordTitleColumn(open),
    { title: '详情', key: 'summary', ellipsis: true, render: (_, row) => label(row.content ?? row.body ?? row.detail ?? row.contact ?? row.chat_id) || '—' },
    {
      title: '状态', key: 'status', width: 140,
      render: (_, row) => <Tag color={
        row.audit_status === 'approved' || row.active === true ? 'success'
          : row.audit_status === 'rejected' || row.active === false ? 'error' : 'processing'
      }>
        {stateLabel(row.audit_status ?? row.state ?? row.role ?? (
          row.active === true ? '启用' : row.active === false ? '停用' : row.kind
        ))}
      </Tag>,
    },
  ]
}

export function RecordInformation({ record, statusLabel = '状态', extra = [] }: {
  record: Row
  statusLabel?: string
  extra?: DescriptionsProps['items']
}) {
  return <Descriptions title="基本信息" column={1} bordered items={[
    { key: 'id', label: '记录 ID', children: record.id },
    ...extra,
    { key: 'status', label: statusLabel, children: <Tag>{stateLabel(record.audit_status ?? record.state ?? record.kind)}</Tag> },
    { key: 'created', label: '创建时间', children: formatDate(record.created_at) },
  ]} />
}

export function RecordBody({ record: row }: { record: Row }) {
  return <>
    {row.content && <p className="review-full-text">{label(row.content)}</p>}
    {row.body && <p className="text-content">{label(row.body)}</p>}
    {row.contact && <p>联系方式：{label(row.contact)}</p>}
    {row.visibility && <Tag>{stateLabel(row.visibility)}</Tag>}
    {row.reconciliation && <p>核对结果：{stateLabel(row.reconciliation)} · {label(row.reconciliation_note)}</p>}
    <small>{formatDate(row.created_at)} {row.id}</small>
    {row.detail && <pre>{label(row.detail)}</pre>}
    {row.url && <p><a href={label(row.url)} target="_blank" rel="noreferrer">查看资源链接</a></p>}
  </>
}
