import { useEffect, useRef, useState } from 'react'
import { useQuery, useQueryClient } from '@tanstack/react-query'
import { Alert, Button, Input, Space, Table } from 'antd'
import { workspace, type RecommendPostInput, type RecommendationView } from './api'
import { formatDate } from '../../../shared/format-date'
import { Outbox } from './outbox'
import { PaginationControls } from '../../../shared/pagination-controls'
import { actionColumn } from '../../../shared/row-actions'
import { notifySuccess } from '../../../shared/notify'

export const recommendationStates: Record<RecommendationView['state'], string> = {
  preparing: '准备中',
  processing: '待投递／处理中',
  accepted: '已受理',
  blocked: '处理受阻',
  cancelled: '已取消',
}
const reasons: Record<string, string> = {
  forbidden: '没有本校推荐权限',
  recommendation_not_eligible: '帖子需审核通过、可见，并具有与当前内容匹配的审核证据。',
  recommendation_configuration_required: '来源配置尚不可用或路由已暂停，请联系管理员。',
  recommendation_pending: '已有未完成的来源推送，请查看当前记录；异常时联系平台管理员。',
  recommendation_cooldown: '尚未达到再次推荐间隔，请查看下次可推荐时间。',
}

export function PostRecommendation({
  school,
  id,
  saved,
  autoOpen = false,
  platformAdmin = false,
  onBlockingChange,
}: {
  school: string
  id: string
  saved: () => Promise<void>
  autoOpen?: boolean
  platformAdmin?: boolean
  onBlockingChange?: (blocking: boolean) => void
}) {
  const [open, setOpen] = useState(autoOpen)
  const [page, setPage] = useState(1)
  const [diagnostic, setDiagnostic] = useState('')
  const [reason, setReason] = useState('')
  const [error, setError] = useState('')
  const [busy, setBusy] = useState(false)
  const submitting = useRef(false)
  const request = useRef<RecommendPostInput | null>(null)
  const [uncertain, setUncertain] = useState(false)
  const cache = useQueryClient()
  useEffect(() => {
    onBlockingChange?.(busy || uncertain)
    return () => onBlockingChange?.(false)
  }, [busy, uncertain, onBlockingChange])
  const preview = useQuery({
    queryKey: ['linjian', 'recommendation-preview', school, id],
    queryFn: ({ signal }) => workspace.postManagement.recommendationPreview(school, id, signal),
    retry: false,
  })
  const history = useQuery({
    queryKey: ['linjian', 'recommendations', school, id, page],
    queryFn: ({ signal }) => workspace.postManagement.recommendations(school, id, page, signal),
    refetchInterval: (q) =>
      q.state.data?.items.some((r) => r.state === 'preparing' || r.state === 'processing')
        ? 10000
        : false,
  })
  const p = preview.data
  async function refresh() {
    await Promise.all([preview.refetch(), history.refetch(), saved()])
  }
  async function submit() {
    if (submitting.current) return
    if (!request.current) {
      if (!p?.can_recommend || !p.publication_snapshot_id || !reason.trim()) return
      request.current = {
        revision: p.revision,
        content_revision: p.content_revision,
        publication_snapshot_id: p.publication_snapshot_id,
        reason: reason.trim(),
        idempotency_key: crypto.randomUUID(),
      }
    }
    submitting.current = true
    setBusy(true)
    setError('')
    try {
      await workspace.postManagement.recommend(school, id, request.current)
      request.current = null
      setUncertain(false)
      setOpen(false)
      setReason('')
      notifySuccess('已创建再次推送记录，等待处理')
      await cache
        .invalidateQueries({ queryKey: ['linjian', 'post-management', school] })
        .catch((e: Error) => setError(e.message))
      await refresh().catch((e: Error) => setError(e.message))
    } catch (e) {
      const code = e && typeof e === 'object' && 'code' in e ? String(e.code) : ''
      const status = e && typeof e === 'object' && 'status' in e ? Number(e.status) : 0
      setError(reasons[code] ?? (e as Error).message)
      // Only a definitive rejection permits a new request identity. Timeout/5xx retries keep the exact request.
      if (status >= 400 && status < 500) {
        request.current = null
        setUncertain(false)
        await preview.refetch()
      } else setUncertain(true)
    } finally {
      submitting.current = false
      setBusy(false)
    }
  }
  return (
    <Space orientation="vertical" className="full-width">
      {(preview.error || history.error) && (
        <Alert type="error" title={preview.error?.message ?? history.error?.message} />
      )}
      {diagnostic ? (
        <section className="recommendation-diagnostic">
          <Space>
            <h3>来源诊断</h3>
            <Button onClick={() => setDiagnostic('')}>返回推送</Button>
          </Space>
          <Outbox
            key={`${school}:${diagnostic}`}
            school={school}
            initialEvent={diagnostic}
            inlineDetail
          />
        </section>
      ) : (
        <>
          <Space>
            {p?.authorized && (
              <Button
                disabled={!p.can_recommend || preview.isFetching}
                onClick={() => {
                  setError('')
                  setOpen(true)
                  void preview.refetch()
                }}
              >
                {p.latest ? '重推' : '推送'}
              </Button>
            )}
            <Button onClick={() => void refresh()} loading={preview.isFetching || history.isFetching}>
              刷新推送记录
            </Button>
          </Space>
          {open && (
            <section className="recommendation-confirm">
              <h3>{p?.latest ? '再次推送' : '推送'}</h3>
              {error && <Alert type="error" title={error} />}
              {uncertain && (
                <Alert
                  type="warning"
                  title="请求结果尚未确认，请核对原请求；保留原请求身份，不会重复创建。"
                />
              )}
              <p>
                学校：{p?.school_name}；标题：{p?.title}
              </p>
              <p>审核摘要：{p?.summary ?? '暂无有效审核摘要'}</p>
              <p>
                最近推送：
                {p?.latest
                  ? `${formatDate(p.latest.created_at)} ${recommendationStates[p.latest.state]}`
                  : '尚无推送记录'}
              </p>
              <p>将向本校当前消息路由提交一条新的来源消息。WeKeeper 受理不代表实际发送成功。</p>
              <Input.TextArea
                aria-label="再次推荐原因"
                value={reason}
                disabled={busy || uncertain}
                maxLength={500}
                onChange={(e) => setReason(e.target.value)}
                placeholder="请填写人工推荐原因"
              />
              <Space>
                <Button disabled={busy || uncertain} onClick={() => setOpen(false)}>
                  取消
                </Button>
                <Button
                  type="primary"
                  loading={busy}
                  disabled={
                    !uncertain && (!p?.can_recommend || !reason.trim() || preview.isFetching)
                  }
                  onClick={() => void submit()}
                >
                  {uncertain ? '核对原请求' : p?.latest ? '确认重推' : '确认推送'}
                </Button>
              </Space>
            </section>
          )}
          {p?.unavailable_reason && <p>{reasons[p.unavailable_reason] ?? '当前无法推荐'}</p>}
          {p?.next_eligible_at && <p>下次可推荐时间：{formatDate(p.next_eligible_at)}</p>}
          <p>质量由学校管理员、运营人员人工判断。已受理不代表实际发送成功。</p>
          <Table<RecommendationView>
            rowKey="id"
            tableLayout="fixed"
            size="small"
            pagination={false}
            scroll={{ x: platformAdmin ? 1150 : 1030 }}
            dataSource={history.data?.items ?? []}
            columns={[
              { title: '发起时间', width: 170, render: (_, r) => formatDate(r.created_at) },
              {
                title: '发起类型',
                width: 100,
                render: (_, r) =>
                  r.trigger_type === 'manual_recommendation' ? '人工推荐' : '审核自动推送',
              },
              { title: '状态', width: 110, render: (_, r) => recommendationStates[r.state] },
              {
                title: '受理时间',
                width: 170,
                render: (_, r) => (r.accepted_at ? formatDate(r.accepted_at) : '—'),
              },
              { title: '操作者', width: 120, render: (_, r) => r.actor_name ?? '审核发布流程' },
              { title: '推荐原因', width: 200, render: (_, r) => r.reason ?? '—' },
              { title: '终止说明', width: 160, render: (_, r) => r.terminated_reason ?? '—' },
              ...(platformAdmin
                ? [
                    actionColumn<RecommendationView>({
                      width: 120,
                      render: (r) => (
                        <Button
                          type="link"
                          aria-label={`查看来源诊断 ${formatDate(r.created_at)}`}
                          onClick={() => setDiagnostic(r.id)}
                        >
                          查看来源诊断
                        </Button>
                      ),
                    }),
                  ]
                : []),
            ]}
          />
          <PaginationControls
            page={page}
            more={history.data?.page?.has_more}
            busy={history.isFetching}
            change={setPage}
          />
        </>
      )}
    </Space>
  )
}
