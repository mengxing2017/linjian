import { useState } from 'react'
import { Alert, Button, Select, Space, Tag } from 'antd'
import { workspace, type School } from './api'
import { formatDate } from '../../../shared/format-date'
import { label } from '../../../shared/label'
import { useDialogs } from '../../../shared/use-dialogs'
import { RecordBody, RecordInformation, RecordList, recordTitleColumn, type RecordDetailProps } from './record-list'
import { states, stateLabel } from './state-label'

export function CommentReviews({ school, refresh }: { school: School; refresh: () => Promise<void> }) {
  const [status, setStatus] = useState('pending')
  return <RecordList school={school.id} resource="comments" scrollWidth={1040}
    status={status} onStatusChange={setStatus}
    filters={change => <Select aria-label="审核状态" value={status} style={{ width: 140 }} onChange={change} options={[
      { value: '', label: '全部状态' },
      ...['pending', 'approved', 'rejected'].map(value => ({ value, label: states[value] })),
    ]} />}
    columns={open => [recordTitleColumn(open),
      { title: '审核状态', key: 'status', width: 140, render: (_, row) => <Tag color={row.audit_status === 'approved' ? 'success' : row.audit_status === 'rejected' ? 'error' : 'processing'}>{stateLabel(row.audit_status)}</Tag> },
      { title: '内容', key: 'summary', render: (_, row) => <p className="review-full-text">{label(row.content ?? row.body) || '—'}</p> },
      { title: '用户昵称', key: 'author', dataIndex: 'author_name', width: 160, ellipsis: true, render: value => label(value) || '—' },
    ]}
    detail={props => <CommentReviewDetail {...props} school={school} refresh={refresh} />}
  />
}

function CommentReviewDetail({ record, close, reload, school, refresh }: RecordDetailProps & { school: School; refresh: () => Promise<void> }) {
  const [error, setError] = useState('')
  const [busy, setBusy] = useState(false)
  const [conflict, setConflict] = useState(false)
  const dialogs = useDialogs()
  async function decide(decision: string) {
    const reason = await dialogs.prompt('审核说明')
    if (reason === null) return
    setBusy(true)
    setError('')
    try {
      await workspace.mutate(school.id, `comments/${record.id}/moderation-decisions`, { revision: record.revision, decision, reason })
      await refresh()
      close()
    } catch (error) {
      setError((error as Error).message)
      if (error && typeof error === 'object' && 'status' in error && error.status === 409) setConflict(true)
    } finally {
      setBusy(false)
    }
  }
  return <>
    {dialogs.contextHolder}
    <RecordInformation record={record} statusLabel="审核状态" extra={[
      { key: 'author', label: '用户昵称', children: label(record.author_name) || '—' },
    ]} />
    <section className="detail-section">
      {error && <Alert type="error" showIcon title={error} />}
      {conflict && <>
        <Alert type="warning" title="内容已变更，请重新加载并核对全文。" />
        <Button onClick={() => { void reload().then(success => { if (success) { setConflict(false); setError('') } }) }}>重新加载评论全文</Button>
      </>}
      <RecordBody record={record} />
      <p>审核来源：{record.decision_source === 'automatic' ? '自动审核' : record.decision_source === 'human' ? '人工审核' : record.decision_source === 'legacy' ? '历史记录' : '待审核'} · {formatDate(record.decided_at)}</p>
      <Space wrap>{['approved', 'rejected', 'pending'].map(decision => <Button type="link" danger={decision === 'rejected'} key={decision}
        disabled={busy || conflict || school.status !== 'normal'} onClick={() => void decide(decision)}>{states[decision]}</Button>)}</Space>
    </section>
  </>
}
