import { useState } from 'react'
import { useQuery } from '@tanstack/react-query'
import { Alert, Button, Drawer, Input, InputNumber, Modal, Select, Space, Switch, Table, Tag } from 'antd'
import { operationsApi as api, workspace, type Module, type Filters, type OperationResource, type OperationInput, type ResourceAction } from './api'
import { MediaPicker } from './media-picker'
const strategyLabels = { always: '每次进入都弹出', daily: '每天首次弹出', interval: '按间隔天数弹出' }
const actionLabels = { none: '不跳转', internal_page: '打开站内页面', external_link: '复制外部链接', phone: '拨打电话', copy_text: '复制内容' }
const localDate = (v: string | null) => v ? new Date(new Date(v).getTime() - new Date(v).getTimezoneOffset() * 60000).toISOString().slice(0, 16) : ''
const time = (v: string | null) => v ? new Date(v).toLocaleString() : '不限制'
function target(v: ResourceAction): string { switch (v.kind) { case 'none': return '—'; case 'internal_page': return v.page === 'post' ? `帖子：${v.object_id}` : ({ feed: '首页', resources: '校园服务', legal: '学校说明' }[v.page]); case 'external_link': return v.url; case 'phone': return v.number; case 'copy_text': return v.text } }
export function OperationResources({ school, platformAdmin, schoolAdmin, module }: { school?: string; platformAdmin: boolean; schoolAdmin: boolean; module: Module }) {
  const [scope, setScope] = useState(new URLSearchParams(window.location.search).get('scope') === 'platform' || !school ? 'platform' : 'school')
  const [filters, setFilters] = useState<Filters>({ page: 1, size: 20 })
  const [editing, setEditing] = useState<OperationResource>(), [open, setOpen] = useState(false)
  const [error, setError] = useState('')
  const defaults = scope === 'platform', selected = defaults && platformAdmin ? undefined : school
  const canEdit = defaults ? platformAdmin : schoolAdmin
  const q = useQuery({ queryKey: ['linjian', 'operations', module, selected, scope, filters], queryFn: ({ signal }) => api.list(selected, module, { ...filters, defaults: defaults && !platformAdmin }, signal), enabled: platformAdmin || !!school })
  const title = module === 'group-qrcodes' ? '群二维码' : '首页广告位'
  function change(v: Partial<Filters>) { setFilters({ ...filters, ...v, page: 1 }) }
  return <section><h2>{title}</h2>
    <Space wrap>
      <Select aria-label="配置范围" value={scope} options={[...(school ? [{ value: 'school', label: '当前学校' }] : []), { value: 'platform', label: '平台默认' }]} onChange={v => { setScope(v); setFilters({ page: 1, size: 20 }); setOpen(false) }} />
      <Input.Search aria-label="名称查询" placeholder="按名称查询" value={filters.search ?? ''} onChange={e => change({ search: e.target.value })} />
      <Select aria-label="状态筛选" value={filters.status ?? ''} options={[{ value: '', label: '全部状态' }, { value: 'normal', label: '正常' }, { value: 'hidden', label: '隐藏' }]} onChange={status => change({ status })} />
      {module === 'group-qrcodes' ? <Select aria-label="展示时机筛选" value={filters.strategy ?? ''} options={[{ value: '', label: '全部展示时机' }, ...Object.entries(strategyLabels).map(([value, label]) => ({ value, label }))]} onChange={strategy => change({ strategy })} /> : <Select aria-label="跳转类型筛选" value={filters.action ?? ''} options={[{ value: '', label: '全部跳转类型' }, ...Object.entries(actionLabels).map(([value, label]) => ({ value, label }))]} onChange={action => change({ action })} />}
      <label>开始时间自<Input type="datetime-local" value={localDate(filters.starts_at ?? null)} onChange={e => change({ starts_at: e.target.value ? new Date(e.target.value).toISOString() : undefined })} /></label>
      <label>结束时间至<Input type="datetime-local" value={localDate(filters.ends_at ?? null)} onChange={e => change({ ends_at: e.target.value ? new Date(e.target.value).toISOString() : undefined })} /></label>
      <Button onClick={() => setFilters({ page: 1, size: 20 })}>重置筛选</Button><Button onClick={() => void q.refetch()}>刷新</Button>
      {canEdit && <Button type="primary" onClick={() => { setEditing(undefined); setOpen(true) }}>添加{title}</Button>}
    </Space>
    {defaults && <p>平台默认{!platformAdmin ? '配置只读；统计为平台累计。' : '配置作用于所有学校；统计为平台累计。'}</p>}
    {(error || q.error) && <Alert type="error" title={error || q.error?.message} />}
    <Table<OperationResource> rowKey="id" loading={q.isFetching} dataSource={q.data?.items ?? []} pagination={false} scroll={{ x: 1500 }} columns={[
      { title: module === 'group-qrcodes' ? '二维码名称' : '广告标题', dataIndex: 'title' },
      { title: '所属学校', dataIndex: 'school_name' },
      { title: '图片', render: (_, r) => r.image_url ? <img src={r.image_url} alt={r.title} style={{ width: 70, height: 60, objectFit: 'contain' }} /> : '未配置' },
      ...(module === 'group-qrcodes' ? [{ title: '展示时机', render: (_: unknown, r: OperationResource) => `${strategyLabels[r.popup_strategy]}${r.popup_strategy === 'interval' ? `（${r.popup_interval_days}天）` : ''}` }] : [
        { title: '点击后去哪里', render: (_: unknown, r: OperationResource) => actionLabels[r.action.kind] }, { title: '目标内容', render: (_: unknown, r: OperationResource) => target(r.action) },
      ]),
      { title: '状态', render: (_, r) => <><Tag color={r.active ? 'success' : 'default'}>{r.active ? '正常' : '隐藏'}</Tag>{r.needs_repair && <Tag color="warning">待修复</Tag>}</> },
      { title: '开始展示时间', dataIndex: 'starts_at', render: time }, { title: '结束展示时间', dataIndex: 'ends_at', render: time },
      ...(module === 'home-banners' ? [{ title: '曝光量', dataIndex: 'views' }, { title: '点击量', dataIndex: 'clicks' }] : []),
      { title: '操作', render: (_, r) => <Space><Button type="link" onClick={() => { setEditing(r); setOpen(true) }}>{canEdit ? '编辑' : '查看'}</Button>{canEdit && <Button danger type="link" onClick={() => Modal.confirm({ title: `确认删除“${r.title}”？`, content: '删除后立即停止展示，历史统计保留。', onOk: async () => { try { await api.remove(selected, module, r.id, r.revision); await q.refetch() } catch (e) { setError(e instanceof Error ? e.message : '删除失败'); throw e } } })}>删除</Button>}</Space> },
    ]} />
    <Space><Button disabled={(filters.page ?? 1) <= 1 || q.isFetching} onClick={() => setFilters({ ...filters, page: (filters.page ?? 1) - 1 })}>上一页</Button><span>第 {filters.page ?? 1} 页</span><Button disabled={!q.data?.page?.has_more || q.isFetching} onClick={() => setFilters({ ...filters, page: (filters.page ?? 1) + 1 })}>下一页</Button></Space>
    {open && <Drawer open size="large" title={`${canEdit ? editing ? '编辑' : '添加' : '查看'}${title}`} onClose={() => setOpen(false)}>
      <OperationEditor key={`${selected}:${editing?.id ?? 'new'}`} school={selected} module={module} item={editing} readOnly={!canEdit} onSaved={async () => { await q.refetch(); setOpen(false) }} />
    </Drawer>}
  </section>
}
function OperationEditor({ school, module, item, readOnly, onSaved }: { school?: string; module: Module; item?: OperationResource; readOnly: boolean; onSaved: () => Promise<void> }) {
  const [form, setForm] = useState<OperationInput>({ revision: item?.revision ?? 0, title: item?.title ?? '', content: item?.content ?? '', image_ref: item?.image_ref ?? { kind: 'asset', id: '' }, action: module === 'group-qrcodes' ? {kind:'none'} : item?.action ?? { kind: 'none' }, active: item?.active ?? true, sort_order: item?.sort_order ?? 0, starts_at: item?.starts_at ?? null, ends_at: item?.ends_at ?? null, popup_strategy: item?.popup_strategy ?? 'daily', popup_interval_days: item?.popup_interval_days ?? 1 })
  const [preview, setPreview] = useState(false), [busy, setBusy] = useState(false), [error, setError] = useState(''), [postSearch, setPostSearch] = useState('')
  const posts = useQuery({ queryKey: ['linjian', 'operations-posts', school, postSearch], queryFn: () => workspace.list(school!, 'posts', `?status=approved&search=${encodeURIComponent(postSearch)}`), enabled: !!school && form.action.kind === 'internal_page' && form.action.page === 'post' })
  function change(v: Partial<OperationInput>) { setForm({ ...form, ...v }); setPreview(false) }
  const qr = module === 'group-qrcodes', action = form.action
  function validate() {
    if (!form.title.trim() || !form.image_ref.id) throw new Error('请填写名称并选择图片')
    if (form.starts_at && form.ends_at && form.ends_at <= form.starts_at) throw new Error('结束展示时间必须晚于开始')
    if (action.kind === 'internal_page' && action.page === 'post' && !action.object_id) throw new Error('请选择目标帖子')
  }
  return <Space orientation="vertical" className="full-width">
    {error && <Alert type="error" title={error} />}
    <fieldset disabled={readOnly || busy} style={{ border: 0, padding: 0 }}><Space orientation="vertical" className="full-width">
      <label>{qr ? '二维码名称' : '广告标题'}<Input aria-label={qr ? '二维码名称' : '广告标题'} value={form.title} maxLength={100} onChange={e => change({ title: e.target.value })} /></label>
      <label>展示说明<Input.TextArea value={form.content} maxLength={qr ? 255 : 20000} onChange={e => change({ content: e.target.value })} /></label>
      {!readOnly && <MediaPicker school={school} value={form.image_ref.id || null} onChange={id => change({ image_ref: { kind: 'asset', id: id ?? '' } })} label={qr ? '二维码图片' : '广告图片'} />}
      {readOnly && item?.image_url && <img src={item.image_url} alt={item.title} style={{ maxWidth: 250 }} />}
      {qr ? <><label>展示时机<Select value={form.popup_strategy} options={Object.entries(strategyLabels).map(([value, label]) => ({ value, label }))} onChange={popup_strategy => change({ popup_strategy })} /></label>{form.popup_strategy === 'interval' && <label>间隔天数<InputNumber min={1} max={365} precision={0} value={form.popup_interval_days} onChange={v => change({ popup_interval_days: Number(v) })} /></label>}</> : <>
        <label>点击后去哪里<Select value={action.kind === 'internal_page' && action.page === 'post' ? 'post' : action.kind} options={[{ value: 'none', label: '不跳转' }, { value: 'internal_page', label: '打开站内页面' }, ...(school ? [{ value: 'post', label: '打开帖子详情' }] : []), ...(!['none', 'internal_page'].includes(action.kind) ? [{ value: action.kind, label: `${actionLabels[action.kind]}（历史配置）` }] : [])]} onChange={v => change({ action: v === 'none' ? { kind: 'none' } : v === 'post' ? { kind: 'internal_page', page: 'post', object_id: null } : { kind: 'internal_page', page: 'feed' } })} /></label>
        {action.kind === 'internal_page' && action.page !== 'post' && <Select aria-label="目标页面" value={action.page} options={[{ value: 'feed', label: '首页' }, { value: 'resources', label: '校园服务' }, { value: 'legal', label: '学校说明' }]} onChange={page => change({ action: { kind: 'internal_page', page } })} />}
        {action.kind === 'internal_page' && action.page === 'post' && <Select showSearch filterOption={false} aria-label="目标帖子" placeholder="搜索本校已审核帖子" value={action.object_id ?? undefined} onSearch={setPostSearch} loading={posts.isFetching} options={posts.data?.items.filter(v => v.visibility === 'visible').map(v => ({ value: v.id, label: String(v.title) }))} onChange={object_id => change({ action: { ...action, object_id } })} />}
        {!['none', 'internal_page'].includes(action.kind) && <p>目标内容：{target(action)}；可保留此历史配置或明确更换。</p>}
      </>}
      <label>状态 <Switch checked={form.active} checkedChildren="正常" unCheckedChildren="隐藏" onChange={active => change({ active })} /></label>
      <label>开始展示时间<Input type="datetime-local" value={localDate(form.starts_at)} onChange={e => change({ starts_at: e.target.value ? new Date(e.target.value).toISOString() : null })} /></label>
      <label>结束展示时间<Input type="datetime-local" value={localDate(form.ends_at)} onChange={e => change({ ends_at: e.target.value ? new Date(e.target.value).toISOString() : null })} /></label>
      <label>展示顺序（值大优先）<InputNumber precision={0} value={form.sort_order} onChange={v => change({ sort_order: Number(v) })} /></label>
    </Space></fieldset>
    {!readOnly && <>{preview ? <Alert type="info" title={`确认保存：${form.title}`} description={`${form.active ? '正常' : '隐藏'}；${time(form.starts_at)} 至 ${time(form.ends_at)}；${qr ? strategyLabels[form.popup_strategy] : target(action)}`} /> : null}<Button disabled={busy} onClick={() => { try { validate(); setError(''); setPreview(true) } catch (e) { setError((e as Error).message) } }}>预览</Button><Button type="primary" disabled={!preview} loading={busy} onClick={() => { setBusy(true); setError(''); void api.save(school, module, form, item?.id).then(onSaved).catch(e => setError(e instanceof Error ? e.message : '保存失败')).finally(() => setBusy(false)) }}>确认保存</Button></>}
  </Space>
}
