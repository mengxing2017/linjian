import { useEffect, useRef, useState } from 'react'
import { Alert, App, Button, Card, Descriptions, Input, Select, Space, Table } from 'antd'
import { schoolChanges, type SchoolChangeRequest, type SchoolChangeStatus, type SchoolChangeSummary } from './api'
import { DocumentPreview } from './document-preview'

const labels: Record<SchoolChangeStatus, string> = { pending: '待审', approved: '已通过', rejected: '已驳回', cancelled: '已撤回', invalidated: '已失效' }
const blockerLabels: Record<string, string> = { account_unavailable: '账号不可用', school_unavailable: '目标学校停用', source_changed: '来源学校已变化', role_conflict: '存在学校管理角色', document_unavailable: '材料不可用', target_member_revoked: '目标学校成员已撤销，请先核实准入限制', relationship_required: '旧申请未声明在读或新生关系，请告知申请人撤回重提' }
export function SchoolChanges({ reviewerId, accountId }: { reviewerId: string; accountId?: string }) {
  const { modal } = App.useApp()
  const [status, setStatus] = useState<SchoolChangeStatus>('pending')
  const [page, setPage] = useState(1), [more, setMore] = useState(false)
  const [createdFrom, setCreatedFrom] = useState(''), [createdTo, setCreatedTo] = useState('')
  const [rows, setRows] = useState<SchoolChangeSummary[]>([])
  const [detail, setDetail] = useState<SchoolChangeRequest>()
  const [preview, setPreview] = useState('')
  const [reason, setReason] = useState(''), [error, setError] = useState('')
  const [loading, setLoading] = useState(false), [busy, setBusy] = useState(false)
  const [reload, setReload] = useState(0)
  const serial = useRef(0)
  const mounted = useRef(false)
  useEffect(() => {
    mounted.current = true
    return () => { mounted.current = false; serial.current++ }
  }, [])
  useEffect(() => {
    const controller = new AbortController()
    serial.current++
    setDetail(undefined); setPreview(''); setReason(''); setError(''); setRows([]); setLoading(true)
    const from = createdFrom ? new Date(`${createdFrom}T00:00:00`).toISOString() : undefined
    const to = createdTo ? new Date(`${createdTo}T23:59:59.999`).toISOString() : undefined
    void schoolChanges.list({ status, page, account_id: accountId, created_from: from, created_to: to }, controller.signal).then(result => {
      if (!controller.signal.aborted) { setRows(result.items); setMore(result.page.has_more) }
    }).catch(failure => {
      if (!controller.signal.aborted) setError(failure instanceof Error ? failure.message : '查询失败')
    }).finally(() => { if (!controller.signal.aborted) setLoading(false) })
    return () => controller.abort()
  }, [status, page, accountId, createdFrom, createdTo, reload])
  async function open(id: string) {
    const turn = ++serial.current
    setDetail(undefined); setPreview(''); setReason(''); setError('')
    try {
      const result = await schoolChanges.detail(id, true)
      if (mounted.current && turn === serial.current) setDetail(result)
    } catch (failure) {
      if (mounted.current && turn === serial.current) setError(failure instanceof Error ? failure.message : '读取失败')
    }
  }
  async function review(approve: boolean, expected: number) {
    if (!mounted.current || expected !== serial.current || !detail || busy || !reason.trim()) return
    const turn = ++serial.current
    setBusy(true); setError(''); setPreview('')
    try {
      const result = await (approve ? schoolChanges.approve(detail, reason.trim()) : schoolChanges.reject(detail, reason.trim()))
      if (mounted.current && turn === serial.current) { setDetail(result); setReason(''); setReload(value => value + 1) }
    } catch (failure) {
      if (mounted.current && turn === serial.current) {
        setError(failure instanceof Error ? failure.message : '处理失败，请刷新核对结果')
        setDetail(undefined)
      }
    } finally { if (mounted.current) setBusy(false) }
  }
  function confirmReview(approve: boolean) {
    if (!detail) return
    const expected = serial.current
    modal.confirm({ title: approve ? `确认已核验申请人与${detail.to_school_name}的学生关系并换校？` : '确认驳回并告知原因？', onOk: () => review(approve, expected) })
  }
  const blocked = detail?.blocked_reasons.length || detail?.account_id === reviewerId
  return <Space orientation="vertical" style={{ width: '100%' }}>
    <Card title="换校审核">
      <Alert type="info" title="仅接受目标学校在读、新生关系，其他不接受；不是全平台实名认证。禁止审核本人申请。超过截止时间未完成审核由系统自动驳回。" />
      {accountId && <Alert type="info" title={`当前仅查看账号 ${accountId} 的换校记录，可切换申请状态查询历史结果。`} />}
      <Space wrap style={{ marginBlock: 16 }}>
        <Select aria-label="申请状态" value={status} disabled={busy} options={Object.entries(labels).map(([value, label]) => ({ value, label }))} onChange={value => { setStatus(value); setPage(1) }} />
        <label>提交起始日 <Input type="date" aria-label="提交起始日" value={createdFrom} disabled={busy} onChange={event => { setCreatedFrom(event.target.value); setPage(1) }} /></label>
        <label>提交截止日 <Input type="date" aria-label="提交截止日" value={createdTo} disabled={busy} onChange={event => { setCreatedTo(event.target.value); setPage(1) }} /></label>
        <Button disabled={busy} onClick={() => setReload(value => value + 1)}>刷新</Button>
      </Space>
      {error && <Alert type="error" title={error} />}
      <Table rowKey="id" loading={loading} dataSource={rows} pagination={false} scroll={{ x: 1000 }} columns={[
        { title: '申请人账号', dataIndex: 'account_id' },
        { title: '来源学校', dataIndex: 'from_school_name' },
        { title: '目标学校', dataIndex: 'to_school_name' },
        { title: '状态', dataIndex: 'status', render: (value: SchoolChangeStatus) => labels[value] },
        { title: '提交时间', dataIndex: 'submitted_at' },
        { title: '审核截止', dataIndex: 'review_due_at' },
        { title: '操作', fixed: 'right', render: (_, row) => <Button disabled={busy} onClick={() => void open(row.id)}>查看申请</Button> },
      ]} />
      <Space style={{ marginTop: 16 }}><Button disabled={page <= 1 || loading || busy} onClick={() => setPage(value => value - 1)}>上一页</Button><span>第 {page} 页</span><Button disabled={!more || loading || busy} onClick={() => setPage(value => value + 1)}>下一页</Button></Space>
    </Card>
    {detail && <Card title={`申请详情 · ${labels[detail.status]}`} extra={<Button disabled={busy} onClick={() => { serial.current++; setDetail(undefined); setPreview(''); setReason('') }}>关闭详情</Button>}>
      <Descriptions column={1} items={[
        { key: 'relationship', label: '声明学生关系', children: detail.student_relationship === 'enrolled' ? '在读' : detail.student_relationship === 'new_student' ? '新生' : '旧申请未声明' },
        { key: 'deadline', label: '审核截止', children: detail.review_due_at },
        { key: 'schools', label: '学校变更', children: `${detail.from_school_name} → ${detail.to_school_name}` },
        { key: 'name', label: '声明姓名', children: detail.declared_student_name || '已清理' },
        { key: 'number', label: '学生编号', children: detail.declared_student_number || '未填写或已清理' },
        { key: 'reason', label: '申请原因', children: detail.reason || '已清理' },
        { key: 'result', label: '处理结果', children: detail.result_reason || detail.result_reason_code || '待处理' },
        { key: 'version', label: '申请版本', children: detail.revision },
      ]} />
      {detail.result_reason_code === 'review_timeout' && <Alert type="info" title="系统因超期未审核自动驳回，并非人工判定材料不合格；申请人可重新申请。" />}
      {detail.review_guidance && <Alert type="info" title={detail.review_guidance} />}
      {!!detail.blocked_reasons.length && <Alert type="warning" title={detail.blocked_reasons.map(code => blockerLabels[code] || code).join('；')} />}
      {detail.account_id === reviewerId && <Alert type="warning" title="不能审核本人申请" />}
      <Space wrap style={{ marginBlock: 16 }}>{detail.documents.map(document => <Button key={document.id} disabled={!document.available} onClick={() => setPreview(document.id)}>{document.available ? '受控预览证明' : '材料不可用'}</Button>)}</Space>
      {preview && <DocumentPreview key={`${detail.id}:${preview}`} request={detail.id} document={preview} />}
      {detail.history && <Table caption="最近 100 条审核与材料访问记录" size="small" rowKey="id" dataSource={detail.history} pagination={false} columns={[
        { title: '动作', dataIndex: 'action' }, { title: '操作人', dataIndex: 'actor_id', render: value => value || '系统' },
        { title: '原因码', dataIndex: 'reason_code' }, { title: '时间', dataIndex: 'created_at' },
      ]} />}
      {detail.status === 'pending' && <Space orientation="vertical" style={{ width: '100%' }}>
        <Input.TextArea aria-label="审核结论或驳回原因" maxLength={500} value={reason} disabled={busy} placeholder="批准填写审核结论，驳回填写申请人可理解的补正原因；勿复制证件个人信息" onChange={event => setReason(event.target.value)} />
        <Space>
          <Button type="primary" loading={busy} disabled={!!blocked || !reason.trim()} onClick={() => confirmReview(true)}>批准并换校</Button>
          <Button danger disabled={busy || detail.account_id === reviewerId || !reason.trim()} onClick={() => confirmReview(false)}>驳回</Button>
        </Space>
      </Space>}
    </Card>}
  </Space>
}
