import { useState } from 'react'
import { useQuery } from '@tanstack/react-query'
import { Alert, Button, Drawer, Input, InputNumber, Select, Space, Switch, Table, Tabs } from 'antd'
import { serviceApi, type ServiceEntity, type EntityInputs, type TargetVersion, type TargetChannel, type ShortcutInput, type ServiceOptions, type EffectiveService, type Presentation } from './api'
import { MediaPicker } from './media-picker'
import { useSchoolPermission } from '../workspace/permissions-context'
import { LocalDateTimePicker } from '../../../shared/date-picker'
import { useSchools } from '../../school-context'
import { useUnsavedChanges } from '../../../shared/unsaved-changes'
import { useEffect } from 'react'
type RecordValue = Record<string, unknown> & { id?: string; revision?: number }
const names: Record<ServiceEntity, string> = { 'home-shortcuts': '服务入口', 'operation-service-categories': '服务分类', 'operation-service-targets': '服务目标', 'operation-partners': '合作方', 'operation-cooperations': '合作项目', 'marketing-slots': '官方营销位' }
const message = (error: unknown) => error instanceof Error ? error.message : '操作失败'
export function OperationServices({ school, mode = 'shortcuts' }: { school?: string; mode?: 'shortcuts' | 'targets' | 'marketing' }) {
  const entities: ServiceEntity[] = mode === 'targets' ? ['operation-service-targets', 'operation-partners', 'operation-cooperations'] : mode === 'marketing' ? ['marketing-slots'] : ['home-shortcuts', 'operation-service-categories']
  const [entity, setEntity] = useState<ServiceEntity>(entities[0]), [page, setPage] = useState(1), [search, setSearch] = useState(''), [editing, setEditing] = useState<RecordValue>(), [open, setOpen] = useState(false), [error, setError] = useState('')
  const can = useSchoolPermission(), group = entity === 'operation-service-categories' ? 'category' : 'shortcut'
  const platformEntity = !['home-shortcuts', 'operation-service-categories'].includes(entity)
  const writable = platformEntity || can(`operation.${group}.${editing?.id ? 'update' : 'create'}`)
  const query = useQuery({ queryKey: ['linjian', 'operation-services', school, entity, page, search], queryFn: ({ signal }) => serviceApi.list(school, entity, { page, size: 20, search }, signal) })
  const rows = (query.data?.items ?? []) as unknown as RecordValue[]
  async function decide(row: RecordValue,version: TargetVersion, decision: 'submit' | 'approve' | 'reject' | 'withdraw') {
    try { const reason = globalThis.prompt('填写审核或撤回理由'); if (!reason?.trim() || !row.id || row.revision===undefined) return; await serviceApi.decision(row.id, row.revision, version.id, version.revision, decision, reason); await query.refetch() } catch (cause) { setError(message(cause)) }
  }
  return <section><h2>{mode === 'targets' ? '服务目标与合作' : mode === 'marketing' ? '官方营销位' : '首页与更多服务'}</h2>
    <Tabs activeKey={entity} items={entities.map(key => ({ key, label: names[key] }))} onChange={key => { setEntity(key as ServiceEntity); setPage(1); setSearch('') }} />
    <Space wrap><Input.Search aria-label="搜索服务配置" placeholder="搜索" value={search} onChange={event => { setSearch(event.target.value); setPage(1) }} />
      {(platformEntity || can(`operation.${group}.create`)) && <Button type="primary" onClick={() => { setEditing(undefined); setOpen(true) }}>新增{names[entity]}</Button>}
      <Button onClick={() => void query.refetch()}>刷新</Button>
    </Space>
    {(error || query.error) && <Alert type="error" title={error || query.error?.message} />}
    <Table<RecordValue> rowKey={row => String(row.id)} dataSource={rows.slice(0, 20)} loading={query.isFetching} pagination={false} expandable={entity==='operation-service-targets'?{expandedRowRender:row=><TargetHistory versions={row.versions as TargetVersion[]}/>} : undefined} columns={[
      { title: '名称', render: (_, row) => String(row.title ?? row.name ?? ((row.versions as TargetVersion[] | undefined)?.[0]?.config.name) ?? '') },
      { title: '状态', render: (_, row) => row.versions ? (row.versions as TargetVersion[])[0]?.state : row.active ? '已启用' : '已停用' },
      { title: '操作', fixed: 'right', render: (_, row) => <Space wrap>
        <Button onClick={() => { setEditing(row); setOpen(true) }}>{platformEntity || can(`operation.${group}.update`) ? entity === 'operation-service-targets' ? '修改并生成新版本' : '编辑' : '查看'}</Button>
        {entity === 'operation-service-targets' && (row.versions as TargetVersion[]).slice(0, 1).map(version => <Space key={version.id}>
          {version.state === 'draft' && <Button onClick={() => void decide(row, version, 'submit')}>提交审核</Button>}
          {version.state === 'pending' && <><Button onClick={() => void decide(row, version, 'approve')}>批准</Button><Button danger onClick={() => void decide(row, version, 'reject')}>驳回</Button></>}
          {['approved', 'pending'].includes(version.state) && <Button danger onClick={() => void decide(row, version, 'withdraw')}>撤回</Button>}
        </Space>)}
        {!platformEntity && can(`operation.${group}.delete`) && <Button danger onClick={() => { if (!globalThis.confirm('确认删除此配置？')) return; void serviceApi.remove(school, entity, String(row.id), Number(row.revision)).then(() => query.refetch()).catch(cause => setError(message(cause))) }}>删除</Button>}
      </Space> },
    ]} />
    <Space><Button disabled={page === 1} onClick={() => setPage(page - 1)}>上一页</Button><span>第 {page} 页</span><Button disabled={rows.length <= 20} onClick={() => setPage(page + 1)}>下一页</Button></Space>
    {school && mode === 'shortcuts' && can('operation.shortcut.preview') && <SchoolServices school={school} />}
    {open && <Drawer open size="large" title={names[entity]} onClose={() => setOpen(false)}><ServiceEditor key={`${entity}:${editing?.id ?? 'new'}`} school={school} entity={entity} item={editing} readOnly={!writable} saved={async () => { setOpen(false); await query.refetch() }} /></Drawer>}
    {mode === 'marketing' && <Alert type="info" title="官方单元只使用官方组件，不配置第三方链接。学校需单独启用；资格材料不能代替真实接入验收。" />}
  </section>
}
function initial(entity: ServiceEntity, item?: RecordValue): RecordValue {
  if (item) {
    if (entity === 'operation-service-targets') return { ...(item.versions as TargetVersion[])[0].config, revision: item.revision }
    const keys: Record<ServiceEntity, string[]> = {
      'home-shortcuts': ['revision','title','subtitle','badge','image_ref','category_id','cooperation_id','action','active','featured','sort_order','starts_at','ends_at'],
      'operation-service-categories': ['revision','name','active','sort_order'], 'operation-service-targets': [], 'operation-partners': ['revision','name','contact','active'],
      'operation-cooperations': ['school_ids','revision','partner_id','name','model','disclosure','channel_code','active','starts_at','ends_at'],
      'marketing-slots': ['revision','name','ad_unit_id','format','reward_scene','qualification','active'],
    }
    return Object.fromEntries(keys[entity].map(key => [key, item[key]]))
  }
  switch (entity) {
    case 'home-shortcuts': return { revision: 0, title: '', subtitle: '', badge: '', image_ref: { kind: 'asset', id: '' }, category_id: null, cooperation_id: null, action: { kind: 'none' }, active: true, featured: true, sort_order: 0, starts_at: null, ends_at: null }
    case 'operation-service-targets': return { school_ids:[],partner_id:null,cooperation_id:null,revision: 0, name: '', provider: '', purpose: '', disclosure: '', contact: '', complaint: '', authorization: '', qualification: '', channels: [] }
    case 'operation-service-categories': return { revision: 0, name: '', active: true, sort_order: 0 }
    case 'operation-partners': return { revision: 0, name: '', contact: '', active: true }
    case 'operation-cooperations': return { school_ids:[],revision: 0, name: '', partner_id: '', model: 'service', disclosure: '', channel_code: '', active: true, starts_at: null, ends_at: null }
    case 'marketing-slots': return { revision: 0, name: '', ad_unit_id: '', format: 'custom', reward_scene: null, qualification: '', active: false }
  }
}
function ServiceEditor({ school, entity, item, readOnly, saved }: { school?: string; entity: ServiceEntity; item?: RecordValue; readOnly: boolean; saved: () => Promise<void> }) {
  const can=useSchoolPermission()
  const [form, setForm] = useState<RecordValue>(() => initial(entity, item)), [busy, setBusy] = useState(false), [error, setError] = useState('')
  const [previewSchool,setPreviewSchool]=useState(school??''),[previewAt,setPreviewAt]=useState(''),[previewRows,setPreviewRows]=useState<EffectiveService[]>(),[uploading,setUploading]=useState(false)
  const schools=useSchools(), setDirty=useUnsavedChanges()
  useEffect(()=>{setDirty(false);return()=>setDirty(false)},[setDirty])
  const options = useQuery({ queryKey: ['linjian','operation-service-target-options',school], queryFn: () => serviceApi.options(school), enabled: entity === 'home-shortcuts' })
  const partners = useQuery({ queryKey: ['linjian','service-partner-options'], queryFn: () => serviceApi.list(undefined, 'operation-partners', { size: 100 }), enabled: entity === 'operation-cooperations' || entity === 'operation-service-targets' })
  const cooperations = useQuery({queryKey:['linjian','target-cooperation-options'],queryFn:()=>serviceApi.list(undefined,'operation-cooperations',{size:100}),enabled:entity==='operation-service-targets'})
  const patch = (value: Record<string, unknown>) => { setDirty(true); setPreviewRows(undefined); setForm(previous => ({ ...previous, ...value })) }
  const text = (key: string, label: string, multiline = false) => <label key={key}>{label}{multiline ? <Input.TextArea aria-label={label} value={String(form[key] ?? '')} maxLength={2000} onChange={event => patch({ [key]: event.target.value })} /> : <Input aria-label={label} value={String(form[key] ?? '')} maxLength={2000} onChange={event => patch({ [key]: event.target.value })} />}</label>
  async function save() { setBusy(true); setError(''); try { await serviceApi.save(school, entity, form as unknown as EntityInputs[ServiceEntity], item?.id); setDirty(false); await saved() } catch (cause) { setError(message(cause)) } finally { setBusy(false) } }
  async function preview() {if(!previewSchool)return;setBusy(true);setError('');try{const config=await serviceApi.preview(previewSchool);const result=await serviceApi.draftPreview(previewSchool,{token:config.token,at:previewAt?new Date(previewAt).toISOString():new Date().toISOString(),resource_id:item?.id??null,platform_default:!school,draft:form as unknown as ShortcutInput});setPreviewRows(result.items)}catch(cause){setError(message(cause))}finally{setBusy(false)}}
  return <Space orientation="vertical" className="full-width"><fieldset disabled={readOnly || busy || uploading} style={{ border: 0 }}><div className="form-grid">
    {entity === 'home-shortcuts' ? <>
      {text('title','入口名称')}{text('subtitle','副标题')}{text('badge','角标')}
      <MediaPicker school={school} context={item ? 'operation.shortcut.update' : 'operation.shortcut.create'} label="服务图标" onBusyChange={setUploading} value={(form.image_ref as ShortcutInput['image_ref']).id || null} onChange={id => patch({ image_ref: { kind:'asset', id: id ?? '' } })} />
      <label>分类<Select allowClear value={form.category_id as string | undefined} options={options.data?.categories.map(value => ({ value: value.id, label: value.name }))} onChange={value => patch({category_id:value ?? null})} /></label>
      <label>合作项目<Select allowClear value={form.cooperation_id as string | undefined} options={options.data?.cooperations.map(value => ({ value:value.id,label:value.name }))} onChange={value => patch({cooperation_id:value ?? null})} /></label>
      <ShortcutActionEditor action={form.action as ShortcutInput['action']} options={options.data} change={action => patch({ action })} />
      <label>首页精选<Switch checked={Boolean(form.featured)} onChange={value => patch({featured:value})} /></label>
    </> : text('name','名称')}
    {['operation-service-targets','operation-cooperations'].includes(entity)&&<label>适用学校（留空代表明确授权全部学校）<Select mode="multiple" value={(form.school_ids as string[])??[]} options={schools.data?.map(value=>({value:value.id,label:value.name}))} onChange={school_ids=>patch({school_ids})}/></label>}
    {entity === 'operation-service-targets' && <>
      <label>合作方<Select allowClear value={form.partner_id as string|undefined} options={partners.data?.items.map(value=>({value:value.id,label:value.name}))} onChange={value=>patch({partner_id:value??null,cooperation_id:null})}/></label>
      <label>合作项目<Select allowClear value={form.cooperation_id as string|undefined} options={cooperations.data?.items.filter(value=>value.partner_id===form.partner_id).map(value=>({value:value.id,label:value.name}))} onChange={value=>patch({cooperation_id:value??null})}/></label>
      {text('provider','服务提供方')}{text('purpose','用途',true)}{text('disclosure','强制披露',true)}{text('contact','联系渠道')}{text('complaint','投诉渠道')}{text('authorization','授权材料说明',true)}{text('qualification','主体类目和渠道资格',true)}
      <TargetChannels channels={form.channels as TargetChannel[]} change={channels => patch({channels})} onBusyChange={setUploading} />
      <Alert type="info" title="修改会生成新草稿并撤回旧目标；需不同管理员复审，已引用入口不会自动重绑。" />
    </>}
    {entity === 'operation-partners' && text('contact','内部合作联系人')}
    {entity === 'operation-cooperations' && <>
      <label>合作方<Select value={String(form.partner_id)} options={partners.data?.items.map(value=>({value:value.id,label:value.name}))} onChange={value=>patch({partner_id:value})} /></label>
      <label>模式<Select value={String(form.model)} options={['service','sponsor','cps'].map(value=>({value,label:value}))} onChange={value=>patch({model:value})} /></label>
      {text('disclosure','合作披露',true)}{text('channel_code','非个人化渠道代码')}<p>链接参数预览：linjian_channel={String(form.channel_code??'')}；linjian_school=当前学校 ID。不会加入账号或个人信息。二维码保持审核版本中的静态素材。</p>
    </>}
    {entity === 'marketing-slots' && <>{text('ad_unit_id','官方广告单元 ID')}<label>官方形态<Select value={String(form.format)} options={[{value:'custom',label:'原生模板'},{value:'rewarded',label:'自愿激励视频：本次服务区配色'}]} onChange={format=>patch({format,reward_scene:format==='rewarded'?'service_theme':null})}/></label>{text('qualification','官方资格与核验说明',true)}</>}
    {'active' in form && <label>启用<Switch checked={Boolean(form.active)} onChange={value=>patch({active:value})} /></label>}
    {'sort_order' in form && <label>排序<InputNumber value={Number(form.sort_order)} precision={0} onChange={value=>patch({sort_order:value ?? 0})} /></label>}
    {['starts_at','ends_at'].filter(key=>key in form).map(key=><label key={key}>{key==='starts_at'?'开始时间':'结束时间'}<LocalDateTimePicker value={form[key] ? localTime(String(form[key])) : ''} onChange={value=>patch({[key]:value?new Date(value).toISOString():null})} /></label>)}
  </div></fieldset>{error && <Alert type="error" title={error} />}
    {!readOnly && can('operation.shortcut.preview') && entity==='home-shortcuts'&&<Space wrap><label>预览学校<Select style={{minWidth:180}} value={previewSchool||undefined} disabled={!!school} options={schools.data?.map(value=>({value:value.id,label:value.name}))} onChange={setPreviewSchool}/></label><LocalDateTimePicker value={previewAt} onChange={setPreviewAt}/><Button disabled={!previewSchool||busy||uploading} onClick={()=>void preview()}>预览未保存配置</Button></Space>}
    {previewRows&&<Table<EffectiveService> rowKey="id" dataSource={previewRows} pagination={false} columns={[{title:'预览顺序',dataIndex:'title'},{title:'状态',render:(_,row)=>row.hidden_reason??'有效'}]}/>}
    {!readOnly && <Button type="primary" loading={busy} disabled={uploading} onClick={()=>void save()}>保存</Button>}</Space>
}
function TargetHistory({versions}:{versions:TargetVersion[]}) {
  return <Space orientation="vertical" className="full-width">{versions.map((version,index)=><details key={version.id}><summary>版本 {version.version_number} · {version.state} · {version.content_digest}</summary>
    {(['name','provider','purpose','disclosure','school_ids','partner_id','cooperation_id','channels','authorization','qualification'] as const).map(key=><p key={key}><strong>{key}{index+1<versions.length&&JSON.stringify(version.config[key])!==JSON.stringify(versions[index+1].config[key])?'（已变化）':''}</strong>：{typeof version.config[key]==='string'?String(version.config[key]):JSON.stringify(version.config[key])}</p>)}
    <Table rowKey="id" pagination={false} dataSource={version.reviews} columns={[{title:'时间',dataIndex:'created_at'},{title:'审核人',dataIndex:'actor_id'},{title:'动作',dataIndex:'decision'},{title:'理由',dataIndex:'reason'}]}/>
  </details>)}</Space>
}
function localTime(value:string) { const date=new Date(value); return new Date(date.getTime()-date.getTimezoneOffset()*60000).toISOString().slice(0,16) }
function ShortcutActionEditor({ action, options, change }: { action: ShortcutInput['action']; options?: ServiceOptions; change:(action:ShortcutInput['action'])=>void }) {
  const selected=action.kind==='target_channel'?options?.targets.find(value=>value.id===action.version_id):undefined
  return <>
    <label>访问方式<Select value={action.kind} options={[{value:'none',label:'仅说明'},{value:'internal_page',label:'站内页面'},{value:'target_channel',label:'已批准第三方服务'}]} onChange={value=>change(value==='none'?{kind:'none'}:value==='internal_page'?{kind:'internal_page',page:'resources',object_id:null}:{kind:'target_channel',version_id:'',channel:''})} /></label>
    {action.kind==='internal_page' && <><label>目标页面<Select value={action.page} options={['feed','resources','legal','post'].map(value=>({value,label:value}))} onChange={page=>change({...action,page:page as 'feed'|'resources'|'legal'|'post',object_id:null})} /></label>{action.page==='post' && <label>本校已审核帖子 ID<Input value={action.object_id??''} onChange={event=>change({...action,object_id:event.target.value})} /></label>}</>}
    {action.kind==='target_channel' && <><label>批准版本<Select value={action.version_id} options={options?.targets.map(value=>({value:value.id,label:`${value.name} · ${value.provider}`}))} onChange={version_id=>change({kind:'target_channel',version_id,channel:options?.targets.find(value=>value.id===version_id)?.channels[0]?.key??''})} /></label><label>默认渠道<Select value={action.channel} options={selected?.channels.map(value=>({value:value.key,label:value.label}))} onChange={channel=>change({...action,channel})} /></label></>}
  </>
}
function TargetChannels({ channels,change,onBusyChange }: { channels:TargetChannel[];change:(channels:TargetChannel[])=>void;onBusyChange:(busy:boolean)=>void }) {
  function patch(index:number, value:Partial<TargetChannel>) { change(channels.map((channel,position)=>position===index?{...channel,...value}:channel)) }
  return <div className="form-wide"><h3>受控渠道</h3>{channels.map((channel,index)=><Space orientation="vertical" className="full-width" key={index}>
    <label>渠道标识<Input value={channel.key} onChange={event=>patch(index,{key:event.target.value})} /></label><label>按钮名称<Input value={channel.label} onChange={event=>patch(index,{label:event.target.value})} /></label>
    <Select value={channel.action.kind} options={['mini_program','web_view','copy_link','qr_code'].map(value=>({value,label:value}))} onChange={kind=>patch(index,{action:kind==='mini_program'?{kind,app_id:'',path:'pages/'}:kind==='qr_code'?{kind,asset_id:''}:{kind:kind as 'web_view'|'copy_link',url:''}})} />
    {channel.action.kind==='mini_program' && <><Input aria-label="目标 AppID" placeholder="目标 AppID" value={channel.action.app_id} onChange={event=>patch(index,{action:{...channel.action,kind:'mini_program',app_id:event.target.value,path:channel.action.kind==='mini_program'?channel.action.path:''}})} /><Input aria-label="目标路径" placeholder="pages/…" value={channel.action.path} onChange={event=>patch(index,{action:{kind:'mini_program',app_id:channel.action.kind==='mini_program'?channel.action.app_id:'',path:event.target.value}})} /></>}
    {(channel.action.kind==='web_view'||channel.action.kind==='copy_link') && <Input aria-label="业务 HTTPS 链接" placeholder="核验过的 HTTPS 业务链接" value={channel.action.url} onChange={event=>patch(index,{action:{kind:channel.action.kind as 'web_view'|'copy_link',url:event.target.value}})} />}
    {channel.action.kind==='qr_code' && <MediaPicker context="operation.shortcut.create" label="服务二维码" onBusyChange={onBusyChange} value={channel.action.asset_id||null} onChange={id=>patch(index,{action:{kind:'qr_code',asset_id:id??''}})} />}
    <Button danger onClick={()=>change(channels.filter((_,position)=>position!==index))}>移除此渠道</Button>
  </Space>)}<Button disabled={channels.length>=8} onClick={()=>change([...channels,{key:`channel_${channels.length+1}`,label:'访问服务',action:{kind:'copy_link',url:''}}])}>添加渠道</Button></div>
}
function SchoolServices({school}:{school:string}) {
  const can=useSchoolPermission(), [at,setAt]=useState(''), [error,setError]=useState(''), [editing,setEditing]=useState<EffectiveService>(), [presentation,setPresentation]=useState<Presentation>({}), [dragged,setDragged]=useState(''), [selected,setSelected]=useState<string[]>([])
  const query=useQuery({queryKey:['linjian','effective-service-preview',school,at],queryFn:async()=>{
    let page=1;const result=await serviceApi.preview(school,{page,size:100,at:at?new Date(at).toISOString():undefined});const items=[...result.items];let more=result.has_more
    while(more){const next=await serviceApi.preview(school,{page:++page,size:100,at:at?new Date(at).toISOString():undefined});if(next.token!==result.token)throw new Error('配置已变化，请刷新');items.push(...next.items);more=next.has_more;if(items.length>1000)throw new Error('排序集合超过一千项，请缩减配置')}
    return {...result,items}
  }})
  async function run(action:()=>Promise<unknown>){try{await action();await query.refetch()}catch(cause){setError(message(cause))}}
  const options=useQuery({queryKey:['linjian','school-override-options',school],queryFn:()=>serviceApi.options(school)})
  const rows=query.data?.items??[]
  function batch(active:boolean){const items=rows.filter(row=>selected.includes(row.id)).map(row=>({id:row.id,revision:row.school_id?row.revision:row.override_revision,active}));if(!items.length||!query.data)return;void run(async()=>{await serviceApi.batch(school,'batch-status',query.data!.token,items);setSelected([])})}
  function reorder(target:string){if(!dragged||dragged===target||!query.data)return;const ordered=[...rows],from=ordered.findIndex(row=>row.id===dragged),to=ordered.findIndex(row=>row.id===target);if(from<0||to<0)return;const [item]=ordered.splice(from,1);ordered.splice(to,0,item);void run(()=>serviceApi.batch(school,'order',query.data!.token,ordered.map(row=>({id:row.id,revision:row.school_id?row.revision:row.override_revision}))));setDragged('')}
  return <section><h3>学校混排、覆盖与预览</h3><LocalDateTimePicker value={at} onChange={setAt} /><p>拖动行提交完整混排集合；学校覆盖不改变服务目标和强制披露。预览不产生曝光。</p>
    {(error||query.error)&&<Alert type="error" title={error||query.error?.message} />}
    {can('operation.shortcut.update')&&<Space><Button disabled={!selected.length} onClick={()=>batch(true)}>批量启用</Button><Button disabled={!selected.length} onClick={()=>batch(false)}>批量隐藏</Button></Space>}
    <Table<EffectiveService> rowKey="id" dataSource={rows} pagination={false} rowSelection={can('operation.shortcut.update')?{selectedRowKeys:selected,onChange:keys=>setSelected(keys.map(String))}:undefined} onRow={row=>({draggable:can('operation.shortcut.update'),onDragStart:()=>setDragged(row.id),onDragOver:event=>event.preventDefault(),onDrop:()=>reorder(row.id)})} columns={[
      {title:'入口',dataIndex:'title'},{title:'来源',render:(_,row)=>row.school_id?'本校':'平台默认'},{title:'预览状态',render:(_,row)=>row.hidden_reason??'有效'},
      {title:'操作',fixed:'right',render:(_,row)=>can('operation.shortcut.update')&&<Space>
        <Button onClick={()=>void run(()=>serviceApi.batch(school,'batch-status',query.data!.token,[{id:row.id,revision:row.school_id?row.revision:row.override_revision,active:row.hidden_reason==='入口已停用'}]))}>启用/隐藏</Button>
        {!row.school_id&&<Button onClick={()=>{setEditing(row);setPresentation({})}}>展示覆盖</Button>}
        {!row.school_id&&row.override_revision>0&&<Button onClick={()=>void run(()=>serviceApi.reset(school,row.id,row.override_revision))}>恢复默认</Button>}
      </Space>},
    ]}/>
    {editing&&<Drawer open title="学校展示覆盖" onClose={()=>setEditing(undefined)}><p>未填写的字段继承平台值；副标题和角标可显式清空。</p><Space orientation="vertical" className="full-width">
      {(['title','subtitle','badge'] as const).map(key=><label key={key}>{key}<Input value={presentation[key]??''} placeholder={`继承：${editing[key]}`} onChange={event=>setPresentation({...presentation,[key]:event.target.value})}/><Button size="small" onClick={()=>setPresentation(previous=>{const next={...previous};delete next[key];return next})}>继承</Button></label>)}
      <MediaPicker school={school} context="operation.shortcut.update" label="本校图标" value={presentation.image_ref?.id??null} onChange={id=>setPresentation(previous=>{const next={...previous};if(id)next.image_ref={kind:'asset',id};else delete next.image_ref;return next})}/>
      <label>分类覆盖<Select allowClear placeholder="继承" value={presentation.category_id===undefined?undefined:presentation.category_id??'none'} options={[{value:'none',label:'不分类'},...(options.data?.categories.map(value=>({value:value.id,label:value.name}))??[])]} onChange={value=>setPresentation(previous=>{const next={...previous};if(value===undefined)delete next.category_id;else next.category_id=value==='none'?null:value;return next})}/></label>
      <label>首页精选<Select allowClear placeholder="继承" value={presentation.featured===undefined?undefined:String(presentation.featured)} options={[{value:'true',label:'精选'},{value:'false',label:'非精选'}]} onChange={value=>setPresentation(previous=>{const next={...previous};if(value===undefined)delete next.featured;else next.featured=value==='true';return next})}/></label>
      <Button type="primary" onClick={()=>void run(async()=>{await serviceApi.override(school,editing.id,editing.override_revision,query.data!.token,presentation);setEditing(undefined)})}>保存覆盖</Button>
    </Space></Drawer>}
  </section>
}
