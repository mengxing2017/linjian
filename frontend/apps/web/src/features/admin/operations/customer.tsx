import { useState } from 'react'
import { useQuery } from '@tanstack/react-query'
import { Alert, Button, Input, Space, Switch } from 'antd'
import { operationsApi as api, type CustomerConfig, type CustomerInput } from './api'
import { MediaPicker } from './media-picker'
export function CustomerConfiguration() {
  const q = useQuery({ queryKey: ['linjian', 'customer-config'], queryFn: api.customer })
  return <section><h2>客户配置</h2>{q.error && <Alert type="error" title={q.error.message} />}{q.data && <CustomerForm key={q.data.revision} initial={q.data} saved={async () => { await q.refetch() }} />}</section>
}
function CustomerForm({ initial, saved }: { initial: CustomerConfig; saved: () => Promise<void> }) {
  const input: CustomerInput = { revision:initial.revision, enabled:initial.enabled, business_asset_id:initial.business_asset_id, join_asset_id:initial.join_asset_id, service_asset_id:initial.service_asset_id, fallback_asset_id:initial.fallback_asset_id, note:initial.note }
  const [form, setForm] = useState<CustomerInput>(input), [busy, setBusy] = useState(false), [error, setError] = useState(''), [success, setSuccess] = useState(false)
  function change(v: Partial<CustomerInput>) { setForm({ ...form, ...v }); setSuccess(false) }
  return <Space orientation="vertical" className="full-width">
    {error && <Alert type="error" title={error} />}{success && <Alert type="success" title="配置已保存" />}
    <p>控制小程序“我的”页商务合作、申请加盟、平台客服三个入口。专属图片为空时使用通用兜底；投诉反馈入口保持可用。</p>
    <label>显示／隐藏 <Switch checked={form.enabled} checkedChildren="显示" unCheckedChildren="隐藏" onChange={enabled => change({ enabled })} /></label>
    {([['business_asset_id', '商务合作二维码'], ['join_asset_id', '申请加盟二维码'], ['service_asset_id', '平台客服二维码'], ['fallback_asset_id', '通用兜底二维码']] as const).map(([key, label]) => <MediaPicker key={key} value={form[key]} label={label} onChange={id => change({ [key]: id })} />)}
    <label>提示文案<Input.TextArea value={form.note} maxLength={255} placeholder="添加客服微信后，请备注来意" onChange={e => change({ note: e.target.value })} /></label>
    <Button type="primary" loading={busy} onClick={() => { setBusy(true); setError(''); void api.saveCustomer({ revision: form.revision, enabled: form.enabled, business_asset_id: form.business_asset_id, join_asset_id: form.join_asset_id, service_asset_id: form.service_asset_id, fallback_asset_id: form.fallback_asset_id, note: form.note }).then(async () => { setSuccess(true); await saved() }).catch(e => setError(e instanceof Error ? e.message : '保存失败')).finally(() => setBusy(false)) }}>保存配置</Button>
  </Space>
}
