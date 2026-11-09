import { useState } from 'react'
import { Alert, Button, Descriptions, Form, Input, Select, Space } from 'antd'
import { workspace, ApiError, type CaseManagementView, type ResolveCaseInput } from './api'
import type { RecordDetailProps } from './record-list'
import { CaseStateTag } from './case-columns'
import { ReviewImage } from './review-image'
import { formatDate } from '../../../shared/format-date'

export function CaseDetail({ school, record, reload, refresh }: RecordDetailProps<CaseManagementView> & { school: string; refresh: () => Promise<void> }) {
  const [form] = Form.useForm<Omit<ResolveCaseInput, 'revision'>>()
  const [busy, setBusy] = useState(false)
  const [conflict, setConflict] = useState(false)
  const [error, setError] = useState('')
  const save = async (values: Omit<ResolveCaseInput, 'revision'>) => {
    if (busy || conflict) return
    setBusy(true); setError('')
    try {
      await workspace.resolveCase(school, record.id, { ...values, revision: record.revision })
      await refresh()
      await reload()
    } catch (cause) {
      if (cause instanceof ApiError && cause.status === 409) setConflict(true)
      setError(cause instanceof Error ? cause.message : '处理失败')
    } finally { setBusy(false) }
  }
  return <>
    <Descriptions title="基本信息" column={1} bordered items={[
      { key: 'title', label: '标题 / 名称', children: record.title },
      { key: 'reporter', label: '举报人', children: record.reporter_name },
      { key: 'school', label: '学校', children: record.school_name },
      { key: 'state', label: '处理状态', children: <CaseStateTag state={record.state} /> },
      { key: 'created', label: '举报时间', children: formatDate(record.created_at) },
      { key: 'handled', label: '处理时间', children: formatDate(record.handled_at, '—') },
      { key: 'id', label: '记录 ID', children: record.id },
    ]} />
    {record.state !== 'open' && !record.handled_at && <Alert type="info" title="历史记录未留存处理时间" />}
    <section className="detail-section">
      <h3>{record.kind === 'feedback' ? '反馈内容' : '举报原因'}</h3>
      <p className="review-full-text">{record.content}</p>
      {record.contact && <p>联系方式：{record.contact}</p>}
      <Space wrap>{record.images.map((source, index) => <ReviewImage key={source} school={school} source={source} index={index} attachmentLabel="反馈附件" />)}</Space>
      {record.resolution && <><h3>处理说明</h3><p className="review-full-text">{record.resolution}</p></>}
      {error && <Alert type="error" title={error} />}
      {conflict && <Alert type="warning" title="案件已发生变化，草稿已保留，请重新加载后再处理" action={<Button loading={busy} onClick={async () => {
        setBusy(true)
        try { if (await reload()) { setConflict(false); setError('') } } finally { setBusy(false) }
      }}>重新加载案件</Button>} />}
      {record.state === 'open' && <Form form={form} layout="vertical" onFinish={save} disabled={busy || conflict}>
        <Form.Item name="resolution" label="处理说明" rules={[{ required: true, whitespace: true, message: '请填写处理说明' }, { max: 2000 }]}><Input.TextArea rows={4} maxLength={2000} /></Form.Item>
        <Form.Item name="state" label="处理结果" rules={[{ required: true, message: '请选择处理结果' }]}><Select options={[{ value: 'resolved', label: '已处理' }, { value: 'dismissed', label: '已驳回' }]} /></Form.Item>
        <Button type="primary" htmlType="submit" loading={busy} disabled={busy || conflict}>保存</Button>
      </Form>}
    </section>
  </>
}
