import { useEffect, useState } from 'react'
import { useQuery } from '@tanstack/react-query'
import { Alert, Button, Drawer, Image, Input, Modal, Select, Space, Table } from 'antd'
import { operationsApi as api, type Feedback, type Filters } from './api'
const types = { product_suggestion: '产品建议', bug_report: 'bug反馈', other: '其他建议', legacy: '历史反馈' }
const states = { open: '待处理', resolved: '已处理', dismissed: '已驳回' }
export function OperationFeedback({ school, canDelete }: { school: string; canDelete: boolean }) {
  const [filters, setFilters] = useState<Filters>({ page: 1, size: 20 }), [selected, setSelected] = useState<Feedback>(), [error, setError] = useState('')
  const q = useQuery({ queryKey: ['linjian', 'operations-feedback', school, filters], queryFn: ({ signal }) => api.feedback(school, filters, signal) })
  useEffect(() => { const id = new URLSearchParams(window.location.search).get('feedback'); if (id) void api.feedbackDetail(school, id).then(setSelected).catch(e => setError(e.message)) }, [school])
  function change(v: Partial<Filters>) { setFilters({ ...filters, ...v, page: 1 }) }
  function remove(v: Feedback) { Modal.confirm({ title: '确认删除此反馈？', content: '删除后将不再显示，操作会记录审计。', onOk: async () => { try { await api.deleteFeedback(school, v.id, v.revision); setSelected(undefined); await q.refetch() } catch (e) { setError(e instanceof Error ? e.message : '删除失败'); throw e } } }) }
  return <section><h2>投诉反馈</h2><Space wrap>
    <Input aria-label="反馈正文查询" placeholder="问题和建议" value={filters.search ?? ''} onChange={e => change({ search: e.target.value })} />
    <Input aria-label="联系方式查询" placeholder="联系方式" value={filters.contact ?? ''} onChange={e => change({ contact: e.target.value })} />
    <Input aria-label="反馈用户查询" placeholder="用户昵称或ID" value={filters.user ?? ''} onChange={e => change({ user: e.target.value })} />
    <Select aria-label="反馈类型筛选" value={filters.feedback_type ?? ''} options={[{ value: '', label: '全部反馈类型' }, ...Object.entries(types).map(([value, label]) => ({ value, label }))]} onChange={feedback_type => change({ feedback_type })} />
    <Select aria-label="处理状态筛选" value={filters.status ?? ''} options={[{ value: '', label: '全部处理状态' }, ...Object.entries(states).map(([value, label]) => ({ value, label }))]} onChange={status => change({ status })} />
    <label>提交时间自<Input type="datetime-local" onChange={e => change({ starts_at: e.target.value ? new Date(e.target.value).toISOString() : undefined })} /></label>
    <label>提交时间至<Input type="datetime-local" onChange={e => change({ ends_at: e.target.value ? new Date(e.target.value).toISOString() : undefined })} /></label>
    <Button onClick={() => setFilters({ page: 1, size: 20 })}>重置筛选</Button><Button onClick={() => void q.refetch()}>刷新</Button>
  </Space>{(error || q.error) && <Alert type="error" title={error || q.error?.message} />}
    <Table<Feedback> rowKey="id" loading={q.isFetching} dataSource={q.data?.items} pagination={false} scroll={{ x: 1800 }} columns={[
      { title: '反馈类型', render: (_, r) => types[r.feedback_type ?? 'legacy'] },
      { title: '问题和建议', dataIndex: 'content', ellipsis: true, width: 240 }, { title: '联系方式', dataIndex: 'contact' },
      { title: '图片', render: (_, r) => <Image.PreviewGroup>{r.images.map(src => <Image key={src} src={src} width={50} height={50} />)}</Image.PreviewGroup> },
      { title: '反馈用户', render: (_, r) => <span>{r.reporter_name}<br />{r.account_id}</span> },
      { title: '处理状态', render: (_, r) => states[r.state] }, { title: 'IP', render: (_, r) => r.submitted_ip || '未记录' },
      { title: '提交时间', render: (_, r) => new Date(r.created_at).toLocaleString() }, { title: '更新时间', render: (_, r) => new Date(r.updated_at).toLocaleString() },
      { title: '操作', render: (_, r) => <Space><Button type="link" onClick={() => { void api.feedbackDetail(school, r.id).then(setSelected).catch(e => setError(e.message)) }}>详情</Button>{canDelete && <Button danger type="link" onClick={() => remove(r)}>删除</Button>}</Space> },
    ]} />
    <Space><Button disabled={(filters.page ?? 1) <= 1 || q.isFetching} onClick={() => setFilters({ ...filters, page: (filters.page ?? 1) - 1 })}>上一页</Button><span>第 {filters.page ?? 1} 页</span><Button disabled={!q.data?.page?.has_more || q.isFetching} onClick={() => setFilters({ ...filters, page: (filters.page ?? 1) + 1 })}>下一页</Button></Space>
    <Drawer open={!!selected} size="large" title="反馈详情" onClose={() => setSelected(undefined)}>{selected && <Space orientation="vertical"><p>反馈类型：{types[selected.feedback_type ?? 'legacy']}</p><p style={{ whiteSpace: 'pre-wrap', overflowWrap: 'anywhere' }}>{selected.content}</p><p>联系方式：{selected.contact || '未填写'}</p><p>反馈用户：{selected.reporter_name}（{selected.account_id}）</p><p>处理状态：{states[selected.state]}</p><p>IP：{selected.submitted_ip || '未记录'}</p><p>提交时间：{new Date(selected.created_at).toLocaleString()}</p><p>更新时间：{new Date(selected.updated_at).toLocaleString()}</p><Image.PreviewGroup>{selected.images.map(src => <Image key={src} src={src} width={180} />)}</Image.PreviewGroup>{canDelete && <Button danger onClick={() => remove(selected)}>删除反馈</Button>}</Space>}</Drawer>
  </section>
}
