import { LocalDateTimePicker } from '../../../shared/date-picker'
import {useState} from 'react'
import {useQuery} from '@tanstack/react-query'
import {Alert,Button,Input,InputNumber,Select,Space,Switch} from 'antd'
import {product,workspace,type Row,type ResourceInput,type ResourceAction,type BusinessPreview,type PublishedResource} from './api'
import {ResourceCard} from '../../community/resource-card'
const localDate=(s:string|null)=>s?new Date(new Date(s).getTime()-new Date(s).getTimezoneOffset()*60000).toISOString().slice(0,16):''
const kinds=[['service','校园服务'],['legal','法律文本']] as const
export function ResourceEditor({item,school,onSaved}:{item?:Row;school?:string;onSaved:()=>Promise<unknown>}){
 const initial=item as unknown as PublishedResource|undefined
 const [form,setForm]=useState<ResourceInput>({id:initial?.id,revision:initial?.revision??0,kind:initial?.kind??'service',title:initial?.title??'',content:initial?.content??'',image_ref:initial?.image_ref??null,action:initial?.action??{kind:'none'},active:initial?.active??true,sort_order:initial?.sort_order??0,starts_at:initial?.starts_at??null,ends_at:initial?.ends_at??null})
 const image = ''; const [preview,setPreview]=useState<BusinessPreview<ResourceInput>>(),[busy,setBusy]=useState(false),[error,setError]=useState('')
 const posts=useQuery({queryKey:['business',school,'resource-post-options'],queryFn:()=>workspace.list(school!,'posts','?status=approved'),enabled:!!school})
 function change(v:Partial<ResourceInput>){setForm({...form,...v});setPreview(undefined)}
 async function run(action:()=>Promise<unknown>){if(busy)return;setBusy(true);setError('');try{await action()}catch(e){setError(e instanceof Error?e.message:'操作失败')}finally{setBusy(false)}}

 const action=form.action
 function actionKind(kind:ResourceAction['kind']){change({action:kind==='none'?{kind}:kind==='internal_page'?{kind,page:'feed'}:kind==='external_link'?{kind,url:''}:kind==='phone'?{kind,number:''}:{kind,text:''}})}
 return <Space orientation="vertical" className="full-width">{error&&<Alert type="error" title={error}/>}
 {!preview?<>
 <div className="form-grid">
 <label>资源类型<Select aria-label="资源类型" style={{width:'100%'}} value={form.kind} disabled={!!initial||busy} options={kinds.map(([value,label])=>({value,label}))} onChange={kind=>change({kind,image_ref:null,action:{kind:'none'}})}/></label>
 <label>标题<Input aria-label="资源标题" value={form.title} maxLength={200} onChange={e=>change({title:e.target.value})}/></label>
 <label className="form-wide">正文或用途说明<Input.TextArea aria-label="资源正文" value={form.content} onChange={e=>change({content:e.target.value})}/></label>
 {!['tag','blocked_word'].includes(form.kind)&&<>
 <label>点击或使用方式<Select aria-label="资源使用方式" style={{width:'100%'}} value={action.kind} options={[{value:'none',label:'仅展示'},{value:'internal_page',label:'打开本站页面'},{value:'external_link',label:'复制外部链接'},{value:'phone',label:'拨打电话'},{value:'copy_text',label:'复制联系方式或说明'}]} onChange={actionKind}/></label>
 {action.kind==='external_link'&&<label className="form-wide">外部链接<Input aria-label="外部链接" type="url" inputMode="url" placeholder="https://" value={action.url} onChange={e=>change({action:{...action,url:e.target.value}})}/></label>}
 {action.kind==='internal_page'&&<><label className="form-wide">本站页面<Select aria-label="本站页面" style={{width:'100%'}} value={action.page} options={[['feed','校园发现'],['resources','校园服务'],['legal','学校说明'],...(school?[['post','动态详情']]:[])].map(([value,label])=>({value,label}))} onChange={page=>change({action:{kind:'internal_page',page:page as 'feed'|'resources'|'legal'|'post',object_id:page==='post'?action.object_id:null}})}/></label>{action.page==='post'&&<label className="form-wide">目标动态<Select aria-label="目标动态" style={{width:'100%'}} placeholder="选择本校已审核动态" value={action.object_id??undefined} options={posts.data?.items.map(v=>({value:v.id,label:String(v.title)}))} onChange={object_id=>change({action:{...action,object_id}})}/></label>}</>}
 {action.kind==='phone'&&<label className="form-wide">电话号码<Input aria-label="电话号码" type="tel" inputMode="tel" value={action.number} onChange={e=>change({action:{...action,number:e.target.value}})}/></label>}
 {action.kind==='copy_text'&&<label className="form-wide">复制内容<Input.TextArea aria-label="复制内容" value={action.text} onChange={e=>change({action:{...action,text:e.target.value}})}/></label>}
 </>}
 <label>排序<InputNumber value={form.sort_order} onChange={v=>change({sort_order:Number(v)})}/></label>
 <label>启用<Switch checked={form.active} onChange={active=>change({active})}/></label>
 <label>生效时间<LocalDateTimePicker value={localDate(form.starts_at)} onChange={value=>change({starts_at:value?new Date(value).toISOString():null})}/></label>
 <label>失效时间<LocalDateTimePicker value={localDate(form.ends_at)} onChange={value=>change({ends_at:value?new Date(value).toISOString():null})}/></label>
 </div>
 <Button type="primary" loading={busy} onClick={()=>void run(async()=>setPreview(await product.resources.preview(school,form)))}>预览</Button>
 </>:<>
 <p>范围：{school?'当前学校':'平台默认'} · {preview.summary.active?'发布启用':'保存为停用'}。学校同类有效资源优先；两端均按以下图片和使用方式展示。</p>
 <ResourceCard school={school??''} item={{...preview.summary,id:preview.summary.id??preview.id,image_url:image||null}}/>
 <Button onClick={()=>setPreview(undefined)} disabled={busy}>返回编辑</Button><Button type="primary" loading={busy} onClick={()=>void run(async()=>{await product.resources.confirm(school,{preview_id:preview.id,digest:preview.digest});await onSaved()})}>确认发布</Button>
 </>}
 </Space>
}
