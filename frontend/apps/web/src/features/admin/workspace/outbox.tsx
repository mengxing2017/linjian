import {useState} from 'react'
import {useQuery} from '@tanstack/react-query'
import {Alert,Button,Space,Table} from 'antd'
import {workspace,type Row} from './api'
import {formatDate} from '../../../shared/format-date'
import {PaginationControls} from '../../../shared/pagination-controls'
const states:Record<string,string>={received:'已收件',applied:'素材已更新',ignored_stale:'旧版本已忽略',rejected:'收件被拒绝'}
export function Outbox({school}:{school:string}){
 const [page,setPage]=useState(1),[error,setError]=useState(''),[busy,setBusy]=useState(false)
 const q=useQuery({queryKey:['source-outbox',school,page],queryFn:({signal})=>workspace.list(school,'source-outbox',`?page[number]=${page}&page[size]=30`,signal),refetchInterval:15000})
 async function replay(id:string){setBusy(true);setError('');try{await workspace.mutate(school,`source-outbox/${encodeURIComponent(id)}/replays`);await q.refetch()}catch(e){setError((e as Error).message)}finally{setBusy(false)}}
 return <Space orientation="vertical" style={{width:'100%'}}>
 <p>收件表示 WeKeeper 已持久受理，素材更新不等于已经发送。发送结果以回执摘要为准。</p>
 {(error||q.error)&&<Alert type="error" title={error||q.error?.message}/>}
 <Button onClick={()=>void q.refetch()} loading={q.isFetching}>刷新发件记录</Button>
 <Table<Row> rowKey="id" dataSource={q.data?.items??[]} loading={q.isPending} pagination={false} scroll={{x:900}} columns={[
 {title:'事件',dataIndex:'id'},
 {title:'内容',dataIndex:'post_id',render:(id:string)=><a href={`/community/${encodeURIComponent(school)}/posts/${encodeURIComponent(id)}`}>查看内容</a>},
 {title:'版本',dataIndex:'sequence'},
 {title:'变更',dataIndex:'event_type',render:v=>v==='content.upsert'?'发布 / 编辑':'撤回'},
 {title:'收件状态',render:(_,r)=>r.receipt_status?states[String(r.receipt_status)]??String(r.receipt_status):'待收件'},
 {title:'请求次数',dataIndex:'attempts'},
 {title:'路由',dataIndex:'route_id'},
 {title:'阻塞原因',dataIndex:'blocked_reason',render:v=>v??'—'},
 {title:'最近问题',dataIndex:'last_error',render:v=>v??'—'},
 {title:'发送摘要',dataIndex:'receipt_summary',render:v=>{const summary=v && typeof v==='object' && !Array.isArray(v)?v.sending:null;return summary && typeof summary==='object' && !Array.isArray(summary)?`待处理 ${summary.pending??0} · 已发送 ${summary.sent??0} · 失败 ${summary.failed??0} · 待核对 ${summary.unknown??0}`:'暂无发送结果'}},
 {title:'创建时间',dataIndex:'created_at',render:v=>formatDate(v)},
 {title:'操作',render:(_,r)=><Button disabled={busy} onClick={()=>void replay(r.id)}>重试同一事件</Button>},
 ]}/>
 <PaginationControls page={page} more={q.data?.page?.has_more} busy={q.isFetching} change={setPage}/>
 </Space>
}
