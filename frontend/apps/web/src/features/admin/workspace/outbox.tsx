import { useState } from 'react'
import { useQuery } from '@tanstack/react-query'
import { Alert, Button, Descriptions, Drawer, Input, Modal, Select, Space, Table, Typography } from 'antd'
import { workspace, type Row } from './api'
import { formatDate } from '../../../shared/format-date'
import { PaginationControls } from '../../../shared/pagination-controls'
const states: Record<string, string> = { received: '已受理' }
const localStates: Record<string, string> = {
  waiting_binding: '待绑定',
  waiting_configuration: '待配置',
  paused: '已暂停',
  preparing_content: '准备内容',
  ready: '待投递',
  in_flight: '请求中',
  retry_wait: '等待重试',
  blocked: '已阻塞',
  accepted: '已受理',
  cancelled: '已取消',
}
const preparationStates: Record<string, string> = {
  pending: '待准备',
  generating: '生成链接中',
  retry_wait: '等待重试',
  blocked: '准备受阻',
  completed: '准备完成',
  superseded: '已失效',
}
function preparation(row: Row) {
  const p = row.preparation
  return p && typeof p === 'object' && !Array.isArray(p) ? p : undefined
}
export function Outbox({
  school,
  initialEvent,
  inlineDetail = false,
}: {
  school: string
  initialEvent?: string
  inlineDetail?: boolean
}) {
  const [page, setPage] = useState(1),
    [error, setError] = useState(''),
    [busy, setBusy] = useState(false)
  const [terminating, setTerminating] = useState(false)
  const [preparationRevision, setPreparationRevision] = useState<number>()
  const [filter, setFilter] = useState<string>(),
    [selected, setSelected] = useState<string | undefined>(initialEvent),
    [retry, setRetry] = useState<string>(),
    [reason, setReason] = useState('')
  const q = useQuery({
    queryKey: ['linjian', 'source-outbox', school, page, filter],
    queryFn: ({ signal }) =>
      workspace.list(
        school,
        'source-outbox',
        `?page[number]=${page}&page[size]=30${filter ? `&status=${filter}` : ''}`,
        signal,
      ),
    refetchInterval: 15000,
  })
  const detail = useQuery({
    queryKey: ['linjian', 'source-outbox-detail', school, selected],
    queryFn: ({ signal }) => workspace.outboxDetail(school, selected!, signal),
    enabled: !!selected,
  })
  async function replay() {
    if (!retry || !reason.trim()) return
    setBusy(true)
    setError('')
    try {
      if (terminating) await workspace.terminateSource(school, retry, reason.trim())
      else if (preparationRevision !== undefined)
        await workspace.retryPreparation(school, retry, reason.trim(), preparationRevision)
      else await workspace.replaySource(school, retry, reason.trim())
      setRetry(undefined)
      setTerminating(false)
      setReason('')
      await q.refetch()
      if (selected) await detail.refetch()
    } catch (e) {
      setError((e as Error).message)
    } finally {
      setBusy(false)
    }
  }
  const detailBody = (
    <>
      {detail.error && <Alert type="error" title={detail.error.message} />}
      {detail.data && (
        <>
          {detail.data.attempts === 0 &&
            (detail.data.delivery_state === 'blocked' ||
              detail.data.preparation?.state === 'blocked') && (
              <Button
                danger
                onClick={() => {
                  setTerminating(true)
                  setRetry(detail.data!.id)
                  setReason('')
                }}
              >
                核对并终止未尝试事件
              </Button>
            )}
          <Descriptions
            column={1}
            items={[
              {
                key: 'id',
                label: '原事件',
                children: <Typography.Text copyable>{detail.data.id}</Typography.Text>,
              },
              {
                key: 'route',
                label: '原路由',
                children: detail.data.route_id ? (
                  <Typography.Text copyable>{detail.data.route_id}</Typography.Text>
                ) : (
                  '待绑定'
                ),
              },
              {
                key: 'binding',
                label: '绑定修订标识',
                children: String(detail.data.binding_version ?? '—'),
              },
              {
                key: 'freeze',
                label: '正文冻结时间',
                children: formatDate(detail.data.frozen_at),
              },
              { key: 'receipt', label: '回执', children: String(detail.data.receipt_id ?? '—') },
              { key: 'blocked', label: '阻塞原因', children: detail.data.blocked_reason ?? '—' },
              { key: 'error', label: '最近问题', children: detail.data.last_error ?? '—' },
            ]}
          />
          {detail.data.preparation && (
            <Descriptions
              column={1}
              items={[
                {
                  key: 'preparation',
                  label: '内容准备',
                  children: preparationStates[detail.data.preparation.state],
                },
                {
                  key: 'attempts',
                  label: '准备次数',
                  children: detail.data.preparation.attempts,
                },
                {
                  key: 'problem',
                  label: '准备问题',
                  children: detail.data.preparation.error_code ?? '—',
                },
                {
                  key: 'next',
                  label: '下次准备时间',
                  children: formatDate(detail.data.preparation.next_attempt_at),
                },
              ]}
            />
          )}
          {detail.data.message && (
            <>
              <p>完整来源消息（{detail.data.message.characters} / 200 字符）</p>
              <p className="text-content">{detail.data.message.content}</p>
              <p>构造版本：{detail.data.message.version}</p>
            </>
          )}
          {detail.data.publication && (
            <>
              <p>来源摘要（最多 20 字，链接生成前为草稿）：{detail.data.publication.summary}</p>
              <p>摘要规则：{detail.data.publication.summary_algorithm}</p>
              <p>审核修订：{detail.data.publication.approved_post_revision}</p>
              <p>链接类型：{detail.data.publication.link_type ?? '待生成'}</p>
              <p className="text-content">{detail.data.publication.link ?? '链接尚未生成'}</p>
              <p>
                链接到期：
                {detail.data.publication.expires_at
                  ? formatDate(detail.data.publication.expires_at)
                  : detail.data.publication.link
                    ? '永久'
                    : '—'}
              </p>
            </>
          )}
          <p>关联事件</p>
          <Table<Row>
            className="linjian-data-table"
            tableLayout="fixed"
            size="small"
            scroll={{ x: 700 }}
            rowKey="id"
            pagination={false}
            dataSource={
              Array.isArray(detail.data.related_events)
                ? (detail.data.related_events as Row[])
                : []
            }
            columns={[
              { title: '事件', width: 320, dataIndex: 'id' },
              { title: '路由', width: 300, dataIndex: 'route_id' },
              { title: '版本', width: 80, dataIndex: 'sequence' },
            ]}
          />
          <p>恢复记录</p>
          <Table<{ actor_id: string; reason: string; created_at: string }>
            className="linjian-data-table"
            tableLayout="fixed"
            size="small"
            scroll={{ x: 500 }}
            rowKey={(r) => r.created_at}
            pagination={false}
            dataSource={detail.data.recoveries ?? []}
            columns={[
              { title: '原因', width: 320, dataIndex: 'reason' },
              {
                title: '时间',
                width: 180,
                dataIndex: 'created_at',
                render: (v) => formatDate(v),
              },
            ]}
          />
        </>
      )}
    </>
  )
  return (
    <Space orientation="vertical" style={{ width: '100%' }}>
      <p>已受理表示 WeKeeper 已持久接收来源消息，不代表实际发送成功。</p>
      {(error || q.error) && <Alert type="error" title={error || q.error?.message} />}
      <Space>
        <Button onClick={() => void q.refetch()} loading={q.isFetching}>
          刷新
        </Button>
        <Select
          aria-label="状态筛选"
          allowClear
          placeholder="全部状态"
          style={{ width: 200 }}
          value={filter}
          options={Object.entries(localStates).map(([value, label]) => ({ value, label }))}
          onChange={(value) => {
            setFilter(value)
            setPage(1)
          }}
        />
      </Space>
      <Table<Row>
        className="linjian-data-table"
        tableLayout="fixed"
        rowKey="id"
        dataSource={q.data?.items ?? []}
        loading={q.isPending}
        pagination={false}
        scroll={{ x: 2250 }}
        columns={[
          {
            title: '事件',
            width: 320,
            dataIndex: 'id',
            render: (id: string) => (
              <Button className="record-title" type="link" onClick={() => setSelected(id)}>
                {id}
              </Button>
            ),
          },
          {
            title: '内容',
            width: 100,
            dataIndex: 'post_id',
            render: (id: string) => (
              <a href={`/community/${encodeURIComponent(school)}/posts/${encodeURIComponent(id)}`}>
                查看内容
              </a>
            ),
          },
          { title: '版本', width: 80, dataIndex: 'sequence' },
          {
            title: '变更',
            width: 120,
            dataIndex: 'event_type',
            render: () => '发布 / 编辑',
          },
          {
            title: '内容准备',
            width: 130,
            render: (_, r) => preparationStates[String(preparation(r)?.state)] ?? '原有事件',
          },
          {
            title: '来源投递状态',
            width: 150,
            dataIndex: 'delivery_state',
            render: (v) => localStates[String(v)] ?? '—',
          },
          {
            title: '收件状态',
            width: 120,
            render: (_, r) =>
              r.receipt_status
                ? (states[String(r.receipt_status)] ?? String(r.receipt_status))
                : '待收件',
          },
          { title: '请求次数', width: 110, dataIndex: 'attempts' },
          { title: '原路由', ellipsis: true, width: 300, dataIndex: 'route_id' },
          {
            title: '阻塞原因',
            ellipsis: true,
            width: 200,
            dataIndex: 'blocked_reason',
            render: (v) => v ?? '—',
          },
          {
            title: '最近问题',
            ellipsis: true,
            width: 280,
            dataIndex: 'last_error',
            render: (v) => v ?? '—',
          },
          { title: '创建时间', width: 180, dataIndex: 'created_at', render: (v) => formatDate(v) },
          {
            title: '操作',
            width: 160,
            fixed: 'right',
            render: (_, r) => {
              const p = preparation(r)
              return p && p.state !== 'completed' ? (
                <Button
                  disabled={busy || p.recoverable !== true}
                  onClick={() => {
                    setPreparationRevision(Number(p.revision))
                    setRetry(r.id)
                    setReason('')
                  }}
                >
                  重试内容准备
                </Button>
              ) : (
                <Button
                  disabled={busy || r.recoverable !== true}
                  onClick={() => {
                    setPreparationRevision(undefined)
                    setRetry(r.id)
                    setReason('')
                  }}
                >
                  重试同一事件
                </Button>
              )
            },
          },
        ]}
      />
      <PaginationControls
        page={page}
        more={q.data?.page?.has_more}
        busy={q.isFetching}
        change={setPage}
      />
      <Modal
        title={
          terminating
            ? '核对并终止未尝试事件'
            : preparationRevision === undefined
              ? '重试同一事件'
              : '重试内容准备'
        }
        open={!!retry}
        confirmLoading={busy}
        okText={terminating ? '确认安全终止' : '重新调度原事件'}
        okButtonProps={{ disabled: !reason.trim() }}
        onOk={() => void replay()}
        onCancel={() => {
          setRetry(undefined)
          setTerminating(false)
        }}
      >
        <p>
          {terminating
            ? '仅可终止从未尝试、无租约的受阻事件；核对原因将留存审计。结果未知的请求不能通过终止绕过。'
            : '保留原事件、路由、正文和回执身份。'}
        </p>
        <Input.TextArea
          aria-label="恢复原因"
          maxLength={500}
          value={reason}
          onChange={(e) => setReason(e.target.value)}
          placeholder="填写已修复的问题和恢复原因"
        />
      </Modal>
      {selected && (
        inlineDetail ? (
          <section className="outbox-inline-detail">
            <Space>
              <h3>详情</h3>
              <Button onClick={() => setSelected(undefined)}>关闭</Button>
            </Space>
            {detailBody}
          </section>
        ) : (
          <Drawer title="详情" open size="large" onClose={() => setSelected(undefined)}>
            {detailBody}
          </Drawer>
        )
      )}
    </Space>
  )
}
