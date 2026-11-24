import { useState } from 'react'
import { useQuery } from '@tanstack/react-query'
import { Alert, Button, Input, Space, Switch } from 'antd'
import { operationsApi as api, type CustomerConfig, type CustomerInput } from './api'
import { MediaPicker } from './media-picker'
import { notifySuccess } from '../../../shared/notify'
import { useDialogs } from '../../../shared/use-dialogs'
export function CustomerConfiguration() {
  const q = useQuery({ queryKey: ['linjian', 'customer-config'], queryFn: api.customer, refetchOnWindowFocus: false })
  return <section><h2>客户配置</h2>{!q.data && q.error && <><Alert type="error" title={q.error.message} /><Button onClick={() => void q.refetch()}>重试读取配置</Button></>}{q.data && <CustomerForm key={q.data.revision} initial={q.data} saved={async () => { await q.refetch({ throwOnError: true }) }} />}</section>
}
function CustomerForm({ initial, saved }: { initial: CustomerConfig; saved: () => Promise<void> }) {
  const { confirm, contextHolder } = useDialogs()
  const [uploading, setUploading] = useState(0)
  const input: CustomerInput = { revision:initial.revision, enabled:initial.enabled, business_asset_id:initial.business_asset_id, join_asset_id:initial.join_asset_id, service_asset_id:initial.service_asset_id, fallback_asset_id:initial.fallback_asset_id, note:initial.note }
  const [form, setForm] = useState<CustomerInput>(input), [busy, setBusy] = useState(false), [error, setError] = useState('')
  const [conflict, setConflict] = useState(false)
  const [written, setWritten] = useState(false)
  const entries = [['service_asset_id', '平台客服'], ['join_asset_id', '我要加盟'], ['business_asset_id', '商务合作']] as const
  const missing = entries.filter(([key]) => !form[key] && !form.fallback_asset_id).map(([, label]) => label)
  function change(v: Partial<CustomerInput>) { setForm({ ...form, ...v }) }
  async function reload() {
    if (!written && !await confirm('重新加载将放弃当前未保存修改，是否继续？')) return
    setBusy(true)
    try { await saved() }
    catch { setError(written ? '配置已保存，读取最新配置失败，请重试读取' : '读取最新配置失败，请重试') }
    finally { setBusy(false) }
  }
  async function save() {
    if (busy || written || conflict || uploading > 0) return
    setBusy(true)
    setError('')
    try {
      await api.saveCustomer(form)
      setWritten(true)
      notifySuccess('客户配置已保存')
      try { await saved() }
      catch { setError('配置已保存，读取最新配置失败，请重试读取') }
    } catch (cause) {
      if (cause && typeof cause === 'object' && 'status' in cause && cause.status === 409) {
        setConflict(true)
        setError('配置已被其他管理员更新，请重新加载后编辑')
      } else setError(cause instanceof Error ? cause.message : '保存失败')
    } finally { setBusy(false) }
  }
  return <Space orientation="vertical" className="full-width">
    {contextHolder}
    <p>控制小程序“我的”和“个人信息”两处“平台服务”：平台客服、我要加盟、商务合作。专属图片为空时使用通用兜底；投诉反馈入口保持可用。加盟仅为联系咨询。</p>
    <label>显示／隐藏 <Switch disabled={busy || written || conflict} checked={form.enabled} checkedChildren="显示" unCheckedChildren="隐藏" onChange={enabled => change({ enabled })} /></label>
    {form.enabled && missing.length > 0 && <Alert type="warning" title={`${missing.join('、')}未配置联系方式；用户可看到入口，但暂无法查看二维码。仍可保存配置。`} />}
    {!form.enabled && <Alert type="info" title="平台服务三个联系入口将隐藏，不影响投诉反馈和个人资料编辑。" />}
    <fieldset disabled={busy || written || conflict}>
      {entries.map(([key, label]) => <div key={key}><MediaPicker onBusyChange={value => setUploading(count => Math.max(0, count + (value ? 1 : -1)))} value={form[key]} label={`${label}二维码`} onChange={id => change({ [key]: id })} /><p>{label}图片来源：{form[key] ? '专属' : form.fallback_asset_id ? '通用' : '未配置'}</p></div>)}
      <MediaPicker onBusyChange={value => setUploading(count => Math.max(0, count + (value ? 1 : -1)))} value={form.fallback_asset_id} label="通用兜底二维码" onChange={id => change({ fallback_asset_id: id })} />
      <label>提示文案<Input.TextArea value={form.note} maxLength={255} placeholder="添加客服微信后，请备注来意" onChange={e => change({ note: e.target.value })} /></label>
    </fieldset>
    <div className="form-actions form-actions-sticky">
      {error && <Alert type="error" title={error} />}
      <Button type="primary" disabled={uploading > 0 || conflict || written} loading={busy} onClick={() => void save()}>保存配置</Button>
      {(conflict || written) && <Button disabled={busy} onClick={() => void reload()}>{written ? '重试读取最新配置' : '重新加载配置'}</Button>}
    </div>
  </Space>
}
