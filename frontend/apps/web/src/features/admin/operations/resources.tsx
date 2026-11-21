import { useSchoolPermission } from '../workspace/permissions-context'
import { LocalDateTimePicker } from '../../../shared/date-picker'
import { PaginationControls } from '../../../shared/pagination-controls'
import { useState } from 'react'
import { useQuery } from '@tanstack/react-query'
import { Alert, Button, Drawer, Input, InputNumber, Modal, Select, Space, Switch, Table, Tag } from 'antd'
import { operationsApi as api, type Module, type Filters, type OperationResource, type OperationInput, type ResourceAction } from './api'
import { MediaPicker } from './media-picker'
import { actionColumn, RowActions } from '../../../shared/row-actions'
import { notifySuccess } from '../../../shared/notify'
const strategyLabels = { always: '每次进入都弹出', daily: '每天首次弹出', interval: '按间隔天数弹出' }
const actionLabels = { none: '不跳转', internal_page: '打开站内页面', external_link: '复制外部链接', phone: '拨打电话', copy_text: '复制内容' }
const localDate = (v: string | null) => v ? new Date(new Date(v).getTime() - new Date(v).getTimezoneOffset() * 60000).toISOString().slice(0, 16) : ''
const time = (v: string | null) => v ? new Date(v).toLocaleString() : '不限制'
function target(v: ResourceAction): string { switch (v.kind) { case 'none': return '—'; case 'internal_page': return v.page === 'post' ? `帖子：${v.object_id}` : ({ feed: '首页', resources: '校园服务', legal: '学校说明' }[v.page]); case 'external_link': return v.url; case 'phone': return v.number; case 'copy_text': return v.text } }
export function OperationResources({ school, platformAdmin, module }: { school?: string; platformAdmin: boolean; schoolAdmin: boolean; module: Module }) {
  const [scope, setScope] = useState(new URLSearchParams(window.location.search).get('scope') === 'platform' || !school ? 'platform' : 'school')
  const [filters, setFilters] = useState<Filters>({ page: 1, size: 20 })
  const [editing, setEditing] = useState<OperationResource>(), [open, setOpen] = useState(false)
  const [error, setError] = useState('')
  const defaults = scope === 'platform', selected = defaults && platformAdmin ? undefined : school
  const can = useSchoolPermission()
  const prefix = module === 'group-qrcodes' ? 'operation.qrcode' : 'operation.banner'
  const canEdit = defaults ? platformAdmin : can(`${prefix}.update`)
  const canCreate = defaults ? platformAdmin : can(`${prefix}.create`)
  const canDelete = defaults ? platformAdmin : can(`${prefix}.delete`)
  const q = useQuery({ queryKey: ['linjian', 'operations', module, selected, scope, filters], queryFn: ({ signal }) => api.list(selected, module, { ...filters, defaults: defaults && !platformAdmin }, signal), enabled: platformAdmin || !!school })
  const title = module === 'group-qrcodes' ? '群二维码' : '首页广告位'
  function change(v: Partial<Filters>) { setFilters({ ...filters, ...v, page: 1 }) }
  return <section><h2>{title}</h2>
    <Space wrap>
      <Select aria-label="配置范围" style={{ width: 150 }} value={scope} options={[...(school ? [{ value: 'school', label: '当前学校' }] : []), { value: 'platform', label: '平台默认' }]} onChange={v => { setScope(v); setFilters({ page: 1, size: 20 }); setOpen(false) }} />
      <Input.Search aria-label="名称查询" placeholder="按名称查询" value={filters.search ?? ''} onChange={e => change({ search: e.target.value })} />
      <Select aria-label="状态筛选" style={{ width: 140 }} value={filters.status ?? ''} options={[{ value: '', label: '全部状态' }, { value: 'normal', label: '正常' }, { value: 'hidden', label: '隐藏' }]} onChange={status => change({ status })} />
      {module === 'group-qrcodes' ? <Select aria-label="展示时机筛选" style={{ width: 190 }} popupMatchSelectWidth={false} value={filters.strategy ?? ''} options={[{ value: '', label: '全部展示时机' }, ...Object.entries(strategyLabels).map(([value, label]) => ({ value, label }))]} onChange={strategy => change({ strategy })} /> : <Select aria-label="跳转类型筛选" style={{ width: 190 }} popupMatchSelectWidth={false} value={filters.action ?? ''} options={[{ value: '', label: '全部跳转类型' }, ...Object.entries(actionLabels).map(([value, label]) => ({ value, label }))]} onChange={action => change({ action })} />}
      <label>开始时间自<LocalDateTimePicker value={localDate(filters.starts_at ?? null)} onChange={value => change({ starts_at: value ? new Date(value).toISOString() : undefined })} /></label>
      <label>结束时间至<LocalDateTimePicker value={localDate(filters.ends_at ?? null)} onChange={value => change({ ends_at: value ? new Date(value).toISOString() : undefined })} /></label>
      <Button onClick={() => setFilters({ page: 1, size: 20 })}>重置筛选</Button><Button onClick={() => void q.refetch()}>刷新</Button>
      {canCreate && <Button type="primary" onClick={() => { setEditing(undefined); setOpen(true) }}>添加</Button>}
    </Space>
    {defaults && <p>平台默认{!platformAdmin ? '配置只读；统计为平台累计。' : '配置作用于所有学校；统计为平台累计。'}</p>}
    {(error || q.error) && <Alert type="error" title={error || q.error?.message} />}
    <Table<OperationResource> className="linjian-data-table" tableLayout="fixed" rowKey="id" loading={q.isFetching} dataSource={q.data?.items ?? []} pagination={false} scroll={{ x: module === 'group-qrcodes' ? 1340 : 1780 }} columns={[
      { title: module === 'group-qrcodes' ? '名称' : '标题', width: 200, dataIndex: 'title' },
      { title: '所属学校', width: 200, dataIndex: 'school_name' },
      { title: '图片', width: 100, render: (_, r) => r.image_url ? <img src={r.image_url} alt={r.title} style={{ width: 70, height: 60, objectFit: 'contain' }} /> : '未配置' },
      ...(module === 'group-qrcodes' ? [{ title: '展示时机', width: 200, render: (_: unknown, r: OperationResource) => `${strategyLabels[r.popup_strategy]}${r.popup_strategy === 'interval' ? `（${r.popup_interval_days}天）` : ''}` }] : [
        { title: '点击后去哪里', width: 160, render: (_: unknown, r: OperationResource) => actionLabels[r.action.kind] }, { title: '目标内容', width: 280, render: (_: unknown, r: OperationResource) => target(r.action) },
      ]),
      { title: '状态', width: 140, render: (_, r) => <><Tag color={r.active ? 'success' : 'default'}>{r.active ? '正常' : '隐藏'}</Tag>{r.needs_repair && <Tag color="warning">待修复</Tag>}</> },
      { title: '开始展示时间', width: 180, dataIndex: 'starts_at', render: time }, { title: '结束展示时间', width: 180, dataIndex: 'ends_at', render: time },
      ...(module === 'home-banners' ? [{ title: '曝光量', width: 100, dataIndex: 'views' }, { title: '点击量', width: 100, dataIndex: 'clicks' }] : []),
      actionColumn<OperationResource>({ width: 140, render: (r) => <RowActions label={r.title} actions={[
        { key: 'edit', label: canEdit ? '编辑' : '查看', onClick: () => { setEditing(r); setOpen(true) } },
        ...(canDelete ? [{ key: 'delete', label: '删除', danger: true, onClick: () => Modal.confirm({ title: `确认删除“${r.title}”？`, content: '删除后立即停止展示，历史统计保留。', onOk: async () => { try { await api.remove(selected, module, r.id, r.revision); await q.refetch() } catch (e) { setError(e instanceof Error ? e.message : '删除失败'); throw e } } }) }] : []),
      ]} /> }),
    ]} />
    <PaginationControls page={filters.page ?? 1} more={q.data?.page?.has_more} busy={q.isFetching} subject="运营资源" change={page => setFilters({ ...filters, page })} />
    {open && <Drawer open size="large" title={canEdit ? editing ? '编辑' : '添加' : '查看'} onClose={() => setOpen(false)}>
      <OperationEditor key={`${selected}:${editing?.id ?? 'new'}`} school={selected} module={module} item={editing} readOnly={editing ? !canEdit : !canCreate} onSaved={async () => { await q.refetch(); setOpen(false) }} />
    </Drawer>}
  </section>
}
function OperationEditor({ school, module, item, readOnly, onSaved }: { school?: string; module: Module; item?: OperationResource; readOnly: boolean; onSaved: () => Promise<void> }) {
  const [uploading, setUploading] = useState(false)
  const [form, setForm] = useState<OperationInput>({ revision: item?.revision ?? 0, title: item?.title ?? '', content: item?.content ?? '', image_ref: item?.image_ref ?? { kind: 'asset', id: '' }, action: module === 'group-qrcodes' ? {kind:'none'} : item?.action ?? { kind: 'none' }, active: item?.active ?? true, sort_order: item?.sort_order ?? 0, starts_at: item?.starts_at ?? null, ends_at: item?.ends_at ?? null, popup_strategy: item?.popup_strategy ?? 'daily', popup_interval_days: item?.popup_interval_days ?? 1 })
  const [preview, setPreview] = useState(false), [busy, setBusy] = useState(false), [error, setError] = useState(''), [postSearch, setPostSearch] = useState('')
  const posts = useQuery({ queryKey: ['linjian', 'operations-posts', school, postSearch], queryFn: () => api.postOptions(school!, item ? 'operation.banner.update' : 'operation.banner.create', postSearch), enabled: !!school && form.action.kind === 'internal_page' && form.action.page === 'post' })
  function change(v: Partial<OperationInput>) { setForm({ ...form, ...v }); setPreview(false) }
  const qr = module === 'group-qrcodes', action = form.action
  function validate() {
    if (uploading) throw new Error('请等待图片上传完成')
    if (!form.title.trim() || !form.image_ref.id) throw new Error('请填写名称并选择图片')
    if (form.starts_at && form.ends_at && form.ends_at <= form.starts_at) throw new Error('结束展示时间必须晚于开始')
    if (action.kind === 'internal_page' && action.page === 'post' && !action.object_id) throw new Error('请选择目标帖子')
  }
  function save() {
    try {
      validate()
    } catch (e) {
      setError((e as Error).message)
      return
    }
    setBusy(true)
    setError('')
    void api
      .save(school, module, form, item?.id)
      .then(async () => {
        notifySuccess(item ? '运营资源已保存' : '运营资源已创建')
        await onSaved()
      })
      .catch((e) => setError(e instanceof Error ? e.message : '保存失败'))
      .finally(() => setBusy(false))
  }
  return <Space orientation="vertical" className="full-width">
    <fieldset disabled={readOnly || busy} style={{ border: 0, padding: 0 }}><div className="form-grid">
      <label className="form-wide">{qr ? '名称' : '标题'}<Input aria-label={qr ? '名称' : '标题'} value={form.title} maxLength={100} onChange={e => change({ title: e.target.value })} /></label>
      <label className="form-wide">展示说明<Input.TextArea value={form.content} maxLength={qr ? 255 : 20000} onChange={e => change({ content: e.target.value })} /></label>
      <div className="form-wide">
        {!readOnly && <MediaPicker context={`${qr ? 'operation.qrcode' : 'operation.banner'}.${item ? 'update' : 'create'}`} onBusyChange={setUploading} school={school} value={form.image_ref.id || null} onChange={id => change({ image_ref: { kind: 'asset', id: id ?? '' } })} label="图片" />}
        {readOnly && item?.image_url && <img src={item.image_url} alt={item.title} style={{ maxWidth: 250 }} />}
      </div>
      {qr ? (
        <div className="form-pair">
          <label>展示时机<Select style={{width:'100%',minWidth:200}} popupMatchSelectWidth={false} value={form.popup_strategy} options={Object.entries(strategyLabels).map(([value, label]) => ({ value, label }))} onChange={popup_strategy => change({ popup_strategy })} /></label>
          {form.popup_strategy === 'interval' && <label>间隔天数<InputNumber min={1} max={365} precision={0} value={form.popup_interval_days} onChange={v => change({ popup_interval_days: Number(v) })} /></label>}
        </div>
      ) : (
        <>
          <label className="form-wide">点击后去哪里<Select style={{width:'100%'}} popupMatchSelectWidth={false} value={action.kind === 'internal_page' && action.page === 'post' ? 'post' : action.kind} options={[{ value: 'none', label: '不跳转' }, { value: 'internal_page', label: '打开站内页面' }, ...(school ? [{ value: 'post', label: '打开帖子详情' }] : [])]} onChange={v => change({ action: v === 'none' ? { kind: 'none' } : v === 'post' ? { kind: 'internal_page', page: 'post', object_id: null } : { kind: 'internal_page', page: 'feed' } })} /></label>
          {action.kind === 'internal_page' && action.page !== 'post' && <label className="form-wide">目标页面<Select aria-label="目标页面" style={{width:'100%'}} value={action.page} options={[{ value: 'feed', label: '首页' }, { value: 'resources', label: '校园服务' }, { value: 'legal', label: '学校说明' }]} onChange={page => change({ action: { kind: 'internal_page', page } })} /></label>}
          {action.kind === 'internal_page' && action.page === 'post' && <label className="form-wide">目标帖子<Select showSearch filterOption={false} aria-label="目标帖子" style={{width:'100%'}} popupMatchSelectWidth={false} placeholder="搜索本校已审核帖子" value={action.object_id ?? undefined} onSearch={setPostSearch} loading={posts.isFetching} options={posts.data?.items.map(v => ({ value: v.id, label: String(v.title) }))} onChange={object_id => change({ action: { ...action, object_id } })} /></label>}
        </>
      )}
      <div className="form-pair">
        <label>状态 <Switch checked={form.active} checkedChildren="正常" unCheckedChildren="隐藏" onChange={active => change({ active })} /></label>
        <label>展示顺序（值大优先）<InputNumber precision={0} value={form.sort_order} onChange={v => change({ sort_order: Number(v) })} /></label>
      </div>
      <div className="form-pair">
        <label>开始展示时间<LocalDateTimePicker disabled={readOnly || busy} value={localDate(form.starts_at)} onChange={value => change({ starts_at: value ? new Date(value).toISOString() : null })} /></label>
        <label>结束展示时间<LocalDateTimePicker disabled={readOnly || busy} value={localDate(form.ends_at)} onChange={value => change({ ends_at: value ? new Date(value).toISOString() : null })} /></label>
      </div>
    </div></fieldset>
    {!readOnly && <div className="form-actions form-actions-sticky">
      {error && <Alert type="error" title={error} />}
      {preview ? <Alert type="info" title={`预览：${form.title}`} description={`${form.active ? '正常' : '隐藏'}；${time(form.starts_at)} 至 ${time(form.ends_at)}；${qr ? strategyLabels[form.popup_strategy] : target(action)}。预览不会保存，可直接保存。`} /> : null}
      <Space>
        <Button disabled={busy} onClick={() => { try { validate(); setError(''); setPreview(true) } catch (e) { setError((e as Error).message) } }}>预览效果</Button>
        <Button type="primary" loading={busy} onClick={save}>保存</Button>
      </Space>
    </div>}
  </Space>
}
