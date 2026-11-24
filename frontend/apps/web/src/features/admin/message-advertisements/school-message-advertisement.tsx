import { useEffect, useRef, useState } from 'react'
import { Alert, Button, Card, Popconfirm, Space, Table, Typography } from 'antd'
import { useOperationScope } from '../../../shared/use-operation-scope'
import { useUnsavedChanges } from '../../../shared/unsaved-changes'
import { RowActions, actionColumn } from '../../../shared/row-actions'
import { AdvertisementEditor } from './advertisement-editor'
import { messageAdvertisements, advertisementContentError, advertisementStatusLabels, type AdvertisementRevision, type MessageAdvertisement } from './api'

export function SchoolMessageAdvertisement({ school, canManage }: { school: string; canManage: boolean }) {
  return <SchoolAdvertisementForm key={school} school={school} canManage={canManage} />
}

function SchoolAdvertisementForm({ school, canManage }: { school: string; canManage: boolean }) {
  const capture = useOperationScope(school)
  const request = useRef<AbortController | null>(null)
  const locked = useRef(false)
  const [config, setConfig] = useState<MessageAdvertisement | null>(null)
  const [content, setContent] = useState('')
  const [loaded, setLoaded] = useState(false)
  const [refreshing, setRefreshing] = useState(true)
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState('')
  const [conflict, setConflict] = useState(false)
  const [history, setHistory] = useState<AdvertisementRevision[]>([])
  const [page, setPage] = useState(1)
  const [more, setMore] = useState(false)
  const [selected, setSelected] = useState<AdvertisementRevision | null>(null)
  const dirty = useUnsavedChanges()
  const candidate = config?.candidate
  const original = candidate?.content ?? config?.current_revision?.content ?? ''
  useEffect(() => { dirty(content !== original); return () => dirty(false) }, [content, original, dirty])
  async function load(preserve: boolean) {
    request.current?.abort()
    const controller = new AbortController()
    request.current = controller
    const active = capture()
    setRefreshing(true)
    try {
      const result = await messageAdvertisements.current(school, controller.signal)
      if (!active() || controller.signal.aborted) return
      setConfig(result); setLoaded(true); setConflict(false); setError('')
      if (!preserve) setContent(result?.candidate?.content ?? result?.current_revision?.content ?? '')
    } catch (failure) { if (active() && !controller.signal.aborted) setError(String(failure)) }
    finally { if (active() && !controller.signal.aborted) setRefreshing(false) }
  }
  useEffect(() => {
    const controller = new AbortController()
    request.current = controller
    messageAdvertisements.current(school, controller.signal).then(result => {
      if (controller.signal.aborted) return
      setConfig(result); setLoaded(true)
      setContent(result?.candidate?.content ?? result?.current_revision?.content ?? '')
    }).catch(failure => { if (!controller.signal.aborted) setError(String(failure)) }).finally(() => { if (!controller.signal.aborted) setRefreshing(false) })
    return () => { controller.abort(); request.current?.abort() }
  }, [school])
  useEffect(() => {
    if (!config) return
    const controller = new AbortController()
    messageAdvertisements.history(school, config.id, page, controller.signal).then(result => {
      if (!controller.signal.aborted) { setHistory(result.items); setMore(result.page.has_more) }
    }).catch(failure => { if (!controller.signal.aborted) setError(String(failure)) })
    return () => controller.abort()
  }, [school, config, page])
  async function run(action: () => Promise<MessageAdvertisement>) {
    if (!canManage || locked.current || conflict || refreshing) return
    const active = capture()
    if (!active()) return
    locked.current = true; setBusy(true); setError('')
    try {
      const result = await action()
      if (!active()) return
      setConfig(result.deleted_at ? null : result)
      setContent(result.deleted_at ? '' : result.candidate?.content ?? result.current_revision?.content ?? '')
      setSelected(null); setPage(1)
      if (result.deleted_at) { setHistory([]); setMore(false) }
    } catch (failure) {
      if (active()) { setError(String(failure)); if (failure && typeof failure === 'object' && 'status' in failure && failure.status === 409) setConflict(true) }
    } finally { if (active()) { locked.current = false; setBusy(false) } }
  }
  const disabled = busy || refreshing || conflict || !loaded || !canManage
  return <Space orientation="vertical" style={{ width: '100%' }}>
    <Alert type="info" title="仅管理本校消息尾部广告；新增、编辑须平台审核。待审期间继续使用旧版，删除后不再为新生成的消息追加广告。" />
    {error && <Alert type="error" title={error} />}
    {conflict && <Alert type="warning" title="数据已变化，当前文案已保留。请重新加载并核对，再手动保存或提交。" />}
    <Button disabled={busy || refreshing} onClick={() => void load(conflict || content !== original)}>重新加载并核对</Button>
    <Card title="当前使用文案" loading={!loaded && !error}>
      <Typography.Paragraph style={{ whiteSpace: 'pre-wrap' }}>{config?.current_revision?.content ?? '暂无已通过广告，原消息正常推送'}</Typography.Paragraph>
      {config?.current_revision?.decision && <Typography.Text type="secondary">通过时间：{config.current_revision.decision.reviewed_at}</Typography.Text>}
    </Card>
    <Card title={candidate ? `候选文案 · ${advertisementStatusLabels[candidate.status]}` : '新增或编辑候选文案'}>
      <AdvertisementEditor content={content} disabled={disabled || candidate?.status === 'pending'} onChange={setContent} />
      {canManage && <Space wrap style={{ marginTop: 16 }}>
        {candidate?.status !== 'pending' && <Button disabled={disabled || !!advertisementContentError(content) || (!!config && content === original)} loading={busy} onClick={() => void run(() => config ? messageAdvertisements.saveDraft(school, config, content) : messageAdvertisements.create(school, content))}>保存草稿</Button>}
        {config && candidate?.status === 'draft' && <Button type="primary" disabled={disabled || content !== candidate.content} onClick={() => void run(() => messageAdvertisements.submit(school, config, candidate))}>提交平台审核</Button>}
        {config && candidate && <Button disabled={disabled} onClick={() => void run(() => messageAdvertisements.withdraw(school, config, candidate))}>撤回候选文案</Button>}
        {config && <Popconfirm title="确认删除本校广告？" description="内容已确定或已受理的消息不会撤回，后续新生成的消息不再追加广告。" disabled={disabled} onConfirm={() => run(() => messageAdvertisements.remove(school, config))}>
          <Button danger disabled={disabled}>删除广告</Button>
        </Popconfirm>}
      </Space>}
    </Card>
    <Card title="修订与审核历史">
      <Table rowKey="id" dataSource={history} pagination={false} columns={[
        { title: '版本', dataIndex: 'number' }, { title: '状态', dataIndex: 'status', render: (status: AdvertisementRevision['status']) => advertisementStatusLabels[status] },
        { title: '文案', dataIndex: 'content', ellipsis: true }, { title: '提交时间', dataIndex: 'submitted_at' },
        { title: '审核说明', render: (_, row) => row.decision?.reason || '—' },
        actionColumn<AdvertisementRevision>({ width: 100, render: row => <RowActions label={`修订 ${row.number}`} actions={[{ key: 'detail', label: '查看全文', onClick: () => setSelected(row) }]} /> }),
      ]} />
      <Space><Button disabled={page === 1} onClick={() => setPage(value => value - 1)}>上一页</Button><span>第 {page} 页</span><Button disabled={!more} onClick={() => setPage(value => value + 1)}>下一页</Button></Space>
      {selected && <Card title={`修订 ${selected.number} 全文`} extra={<Button onClick={() => setSelected(null)}>关闭全文</Button>}><Typography.Paragraph>{selected.content}</Typography.Paragraph><p>{selected.decision?.reason}</p></Card>}
    </Card>
    {!!config?.recent_usage.length && <Card title="最近消息采用情况"><Table rowKey="event_id" dataSource={config.recent_usage} pagination={false} columns={[
      { title: '事件', dataIndex: 'event_id' }, { title: '结果', dataIndex: 'result', render: (result: string) => ({ applied: '已追加', message_budget_exceeded: '消息剩余字数不足，未追加广告', no_approved_advertisement: '无已通过广告' } as Record<string, string>)[result] },
      { title: '消息内容确定时间', dataIndex: 'frozen_at' },
    ]} /><Typography.Text type="secondary">广告追加成功也不代表已送达。此页仅查看追加结果，不能重试推送或执行发送操作。</Typography.Text></Card>}
  </Space>
}
