import { Button, Space, Tag, Tooltip, type TableColumnsType } from 'antd'
import type { CaseManagementView, CaseState } from './api'
import { formatDate } from '../../../shared/format-date'

export const caseStates: Record<CaseState, string> = { open: '待处理', resolved: '已处理', dismissed: '已驳回' }
export function CaseStateTag({ state }: { state: CaseState }) {
  return <Tag color={state === 'resolved' ? 'success' : state === 'dismissed' ? 'error' : 'processing'}>{caseStates[state]}</Tag>
}
export function caseColumns(open: (id: string) => void): TableColumnsType<CaseManagementView> {
  return [
    { title: '标题 / 名称', key: 'title', width: 240, render: (_, row) => <Space><Button className="record-title" type="link" onClick={() => open(row.id)}>{row.title}</Button>{row.kind === 'feedback' && <Tag>反馈</Tag>}</Space> },
    { title: '举报原因', key: 'content', width: 280, render: (_, row) => <Tooltip title={row.content}>{Array.from(row.content).slice(0, 100).join('')}{Array.from(row.content).length > 100 ? '…' : ''}</Tooltip> },
    { title: '举报人', dataIndex: 'reporter_name', width: 140 },
    { title: '学校', dataIndex: 'school_name', width: 160 },
    { title: '处理状态', key: 'state', width: 120, render: (_, row) => <CaseStateTag state={row.state} /> },
    { title: '举报时间', key: 'created', width: 180, render: (_, row) => formatDate(row.created_at) },
    { title: '处理时间', key: 'handled', width: 180, render: (_, row) => formatDate(row.handled_at, '—') },
  ]
}
