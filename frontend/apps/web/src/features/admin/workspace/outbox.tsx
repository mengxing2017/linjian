import {useState} from 'react'
import {useQuery} from '@tanstack/react-query'
import {Alert,Button,Descriptions,Input,Modal,Select,Space,Table} from 'antd'
import {workspace,type Row} from './api'
import {formatDate} from '../../../shared/format-date'
import {PaginationControls} from '../../../shared/pagination-controls'
const states:Record<string,string>={received:'已收件',applied:'素材已更新',ignored_stale:'旧版本已忽略',rejected:'收件被拒绝'}
const localStates:Record<string,string>={waiting_binding:'待绑定',waiting_configuration:'待配置',paused:'已暂停',ready:'待投递',in_flight:'请求中',retry_wait:'等待重试',blocked:'已阻塞',tracking_receipt:'同步回执',settled:'回执处理结束',superseded:'已由后续变更替代'}
export function Outbox({school}:{school:string}){
 const [page,setPage]=useState(1),[error,setError]=useState(''),[busy,setBusy]=useState(false)
 const [filter,setFilter]=useState<string>(),[selected,setSelected]=useState<string>(),[retry,setRetry]=useState<string>(),[reason,setReason]=useState('')
 const q=useQuery({queryKey:['linjian','source-outbox',school,page,filter],queryFn:({signal})=>workspace.list(school,'source-outbox',`?page[number]=${page}&page[size]=30${filter?`&status=${filter}`:''}`,signal),refetchInterval:15000})
 const detail=useQuery({queryKey:['linjian','source-outbox-detail',school,selected],queryFn:({signal})=>workspace.outboxDetail(school,selected!,signal),enabled:!!selected})
 async function replay(){if(!retry||!reason.trim())return;setBusy(true);setError('');try{await workspace.replaySource(school,retry,reason.trim());setRetry(undefined);setReason('');await q.refetch();if(selected)await detail.refetch()}catch(e){setError((e as Error).message)}finally{setBusy(false)}}
 return <Space orientation="vertical" style={{width:'100%'}}>
 <p>收件表示 WeKeeper 已持久受理，素材更新不等于已经发送。发送结果以回执摘要为准。</p>
 {(error||q.error)&&<Alert type="error" title={error||q.error?.message}/>}
 <Space><Button onClick={()=>void q.refetch()} loading={q.isFetching}>刷新发件记录</Button>
 <Select aria-label="发件状态" allowClear placeholder="全部状态" style={{width:200}} value={filter} options={Object.entries(localStates).map(([value,label])=>({value,label}))} onChange={value=>{setFilter(value);setPage(1)}}/></Space>
 <Table<Row> rowKey="id" dataSource={q.data?.items??[]} loading={q.isPending} pagination={false} scroll={{x:1100}} columns={[
 {title:'事件',dataIndex:'id',render:(id:string)=><Button type="link" onClick={()=>setSelected(id)}>{id}</Button>},
 {title:'内容',dataIndex:'post_id',render:(id:string)=><a href={`/community/${encodeURIComponent(school)}/posts/${encodeURIComponent(id)}`}>查看内容</a>},
 {title:'版本',dataIndex:'sequence'},
 {title:'变更',dataIndex:'event_type',render:v=>v==='content.upsert'?'发布 / 编辑':'撤回'},
 {title:'投递状态',dataIndex:'delivery_state',render:v=>localStates[String(v)]??'—'},
 {title:'收件状态',render:(_,r)=>r.receipt_status?states[String(r.receipt_status)]??String(r.receipt_status):'待收件'},
 {title:'请求次数',dataIndex:'attempts'},
 {title:'原路由',dataIndex:'route_id'},
 {title:'阻塞原因',dataIndex:'blocked_reason',render:v=>v??'—'},
 {title:'最近问题',dataIndex:'last_error',render:v=>v??'—'},
 {title:'发送摘要',dataIndex:'receipt_summary',render:v=>{const summary=v && typeof v==='object' && !Array.isArray(v)?v.sending:null;return summary && typeof summary==='object' && !Array.isArray(summary)?`待处理 ${summary.pending??0} · 已发送 ${summary.sent??0} · 失败 ${summary.failed??0} · 待核对 ${summary.unknown??0}`:'暂无发送结果'}},
 {title:'创建时间',dataIndex:'created_at',render:v=>formatDate(v)},
 {title:'操作',render:(_,r)=><Button disabled={busy||r.recoverable!==true} onClick={()=>{setRetry(r.id);setReason('')}}>重试同一事件</Button>},
 ]}/>
 <PaginationControls page={page} more={q.data?.page?.has_more} busy={q.isFetching} change={setPage}/>
 <Modal title="重试同一事件" open={!!retry} confirmLoading={busy} okText="重新调度原事件" okButtonProps={{disabled:!reason.trim()}} onOk={()=>void replay()} onCancel={()=>setRetry(undefined)}>
 <p>保留原事件、路由、正文和回执身份。</p><Input.TextArea aria-label="恢复原因" maxLength={500} value={reason} onChange={e=>setReason(e.target.value)} placeholder="填写已修复的问题和恢复原因"/>
 </Modal>
 <Modal title="发件详情" open={!!selected} footer={null} onCancel={()=>setSelected(undefined)}>
 {detail.error&&<Alert type="error" title={detail.error.message}/>}
 {detail.data&&<>
 <Descriptions column={1} items={[
 {key:'id',label:'原事件',children:detail.data.id},
 {key:'route',label:'原路由',children:String(detail.data.route_id??'待绑定')},
 {key:'binding',label:'绑定修订标识',children:String(detail.data.binding_version??'—')},
 {key:'freeze',label:'正文冻结时间',children:formatDate(detail.data.frozen_at)},
 {key:'receipt',label:'回执',children:String(detail.data.receipt_id??'—')},
 ]}/>
 <p>关联事件</p><Table<Row> size="small" rowKey="id" pagination={false} dataSource={Array.isArray(detail.data.related_events)?detail.data.related_events as Row[]:[]} columns={[{title:'事件',dataIndex:'id'},{title:'路由',dataIndex:'route_id'},{title:'版本',dataIndex:'sequence'}]}/>
 <p>恢复记录</p><Table<{actor_id:string;reason:string;created_at:string}> size="small" rowKey={r=>r.created_at} pagination={false} dataSource={detail.data.recoveries??[]} columns={[{title:'原因',dataIndex:'reason'},{title:'时间',dataIndex:'created_at',render:v=>formatDate(v)}]}/>
 </>}
 </Modal>
 </Space>
}
