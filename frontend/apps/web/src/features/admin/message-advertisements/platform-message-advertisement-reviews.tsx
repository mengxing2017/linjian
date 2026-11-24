import { useEffect, useRef, useState } from 'react'
import { Alert, Button, Card, Input, Popconfirm, Select, Space, Table, Typography } from 'antd'
import { useSchools } from '../../school-context'
import { useOperationScope } from '../../../shared/use-operation-scope'
import { RowActions, actionColumn } from '../../../shared/row-actions'
import { messageAdvertisements, advertisementStatusLabels, type AdvertisementRevision, type AdvertisementStatus } from './api'

export function PlatformMessageAdvertisementReviews() {
  const schools = useSchools()
  const [school, setSchool] = useState<string>()
  const [status, setStatus] = useState<AdvertisementStatus>('pending')
  const [from, setFrom] = useState('')
  const [to, setTo] = useState('')
  const [page, setPage] = useState(1)
  const [refresh, setRefresh] = useState(0)
  const [items, setItems] = useState<AdvertisementRevision[]>([])
  const [more, setMore] = useState(false)
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState('')
  const [selected, setSelected] = useState<string | null>(null)
  useEffect(() => {
    const controller = new AbortController()
    setLoading(true); setError('')
    messageAdvertisements.list({ school_id: school, status, page, created_from: from ? new Date(from).toISOString() : undefined, created_to: to ? new Date(to).toISOString() : undefined }, controller.signal).then(result => {
      if (!controller.signal.aborted) { setItems(result.items); setMore(result.page.has_more) }
    }).catch(failure => { if (!controller.signal.aborted) setError(String(failure)) }).finally(() => { if (!controller.signal.aborted) setLoading(false) })
    return () => controller.abort()
  }, [school, status, page, from, to, refresh])
  return <Space orientation="vertical" style={{ width: '100%' }}>
    <Alert type="info" title="学校消息尾部广告审核。请打开详情核对全文与当前文案，审核不修改帖子，也不补发历史消息。" />
    {error && <Alert type="error" title={error} />}
    <Space wrap>
      <Select aria-label="广告所属学校" allowClear placeholder="全部学校" style={{ minWidth: 180 }} value={school} options={schools.data?.map(item => ({ value: item.id, label: item.name }))} onChange={value => { setSchool(value); setPage(1); setSelected(null) }} />
      <Select aria-label="广告审核状态" value={status} options={Object.entries(advertisementStatusLabels).map(([value, label]) => ({ value, label }))} onChange={value => { setStatus(value); setPage(1); setSelected(null) }} />
      <Input aria-label="提交时间起点" type="datetime-local" value={from} onChange={event => { setFrom(event.target.value); setPage(1) }} />
      <Input aria-label="提交时间终点" type="datetime-local" value={to} onChange={event => { setTo(event.target.value); setPage(1) }} />
      <Button onClick={() => setRefresh(value => value + 1)}>刷新审核队列</Button>
    </Space>
    <Table rowKey="id" dataSource={items} loading={loading} pagination={false} columns={[
      { title: '学校', dataIndex: 'school_name' }, { title: '版本', dataIndex: 'number' },
      { title: '状态', dataIndex: 'status', render: (value: AdvertisementStatus) => advertisementStatusLabels[value] },
      { title: '文案摘要', dataIndex: 'content', ellipsis: true }, { title: '提交时间', dataIndex: 'submitted_at' },
      actionColumn<AdvertisementRevision>({ width: 100, render: row => <RowActions label={`${row.school_name} 修订 ${row.number}`} actions={[{ key: 'detail', label: '查看审核详情', onClick: () => setSelected(row.id) }]} /> }),
    ]} />
    <Space><Button disabled={page === 1} onClick={() => setPage(value => value - 1)}>上一页</Button><span>第 {page} 页</span><Button disabled={!more} onClick={() => setPage(value => value + 1)}>下一页</Button></Space>
    {selected && <AdvertisementReviewDetail key={selected} revision={selected} onClose={() => setSelected(null)} onDone={() => { setSelected(null); setRefresh(value => value + 1) }} />}
  </Space>
}

export function AdvertisementReviewDetail({ revision, onClose, onDone }: { revision: string; onClose: () => void; onDone: () => void }) {
  const capture = useOperationScope(revision)
  const locked = useRef(false)
  const [detail, setDetail] = useState<AdvertisementRevision | null>(null)
  const [reason, setReason] = useState('')
  const [error, setError] = useState('')
  const [busy, setBusy] = useState(false)
  const [stale, setStale] = useState(false)
  const [refresh, setRefresh] = useState(0)
  useEffect(() => {
    const controller = new AbortController()
    messageAdvertisements.detail(revision, controller.signal).then(value => {
      if (!controller.signal.aborted) { setDetail(value); setStale(false); setError('') }
    }).catch(failure => { if (!controller.signal.aborted) setError(String(failure)) })
    return () => controller.abort()
  }, [revision, refresh])
  async function decide(action: 'approve' | 'reject' | 'remove') {
    if (!detail || stale || locked.current || !detail.configuration || detail.config_deleted_at) return
    const active = capture()
    if (!active()) return
    locked.current = true; setBusy(true); setError('')
    try {
      if (action === 'remove') await messageAdvertisements.removePlatform(detail.configuration, reason)
      else await messageAdvertisements.decide(detail, action, reason)
      if (active()) onDone()
    } catch (failure) {
      if (active()) { setError(String(failure)); if (failure && typeof failure === 'object' && 'status' in failure && failure.status === 409) setStale(true) }
    } finally { if (active()) { locked.current = false; setBusy(false) } }
  }
  return <Card title="广告审核详情" extra={<Button disabled={busy} onClick={onClose}>关闭详情</Button>}>
    {error && <Alert type="error" title={error} />}
    {stale && <Alert type="warning" title="修订已变化，说明已保留；重新加载全文并核对后才能继续审核。" />}
    <Button disabled={busy} onClick={() => { setStale(true); setRefresh(value => value + 1) }}>重新加载全文</Button>
    {!detail ? <p>正在加载全文…</p> : <Space orientation="vertical" style={{ width: '100%' }}>
      <p>学校：{detail.school_name}；修订 {detail.number}；状态：{advertisementStatusLabels[detail.status]}</p>
      <p>提交人：{detail.submitted_by ?? detail.created_by}；提交时间：{detail.submitted_at ?? '未提交'}</p>
      <Typography.Text type="secondary">配置版本：{detail.config_version}；修订版本：{detail.version}；审核指纹：{detail.review_digest ?? '尚未提交'}</Typography.Text>
      {detail.config_deleted_at && <Alert type="warning" title="该广告配置已删除或下架，不能继续审批" />}
      <Typography.Title level={5}>当前使用文案</Typography.Title>
      <Typography.Paragraph>{detail.configuration?.current_revision?.content ?? '无'}</Typography.Paragraph>
      <Typography.Title level={5}>本次候选完整文案</Typography.Title>
      <Typography.Paragraph style={{ whiteSpace: 'pre-wrap', overflowWrap: 'anywhere' }}>{detail.content}</Typography.Paragraph>
      <pre aria-label="审核拼接示意" style={{ whiteSpace: 'pre-wrap', overflowWrap: 'anywhere' }}>{`原有完整消息（示例）\n${detail.content}`}</pre>
      <Typography.Text type="secondary">示意不是实际发送预览；原消息加换行及广告超过 200 字符时整段省略广告。</Typography.Text>
      {detail.decision && <p>审核结果：{detail.decision.reason || '通过'}；审核人：{detail.decision.reviewer_id}；时间：{detail.decision.reviewed_at}</p>}
      <Input.TextArea aria-label="广告审核或下架说明" value={reason} disabled={busy} maxLength={500} placeholder="驳回及强制下架必须填写原因" onChange={event => setReason(event.target.value)} />
      <Space wrap>
        {detail.status === 'pending' && <>
          <Button type="primary" disabled={busy || stale || !!detail.config_deleted_at} onClick={() => void decide('approve')}>通过审核</Button>
          <Button danger disabled={busy || stale || !!detail.config_deleted_at || !reason.trim()} onClick={() => void decide('reject')}>驳回广告</Button>
        </>}
        {detail.configuration && !detail.config_deleted_at && <Popconfirm title="确认强制下架？" description="仅停止为后续新生成的消息追加广告，不撤回内容已确定或已受理的消息。" onConfirm={() => decide('remove')} disabled={busy || stale || !reason.trim()}>
          <Button danger disabled={busy || stale || !reason.trim()}>强制下架</Button>
        </Popconfirm>}
      </Space>
    </Space>}
  </Card>
}
