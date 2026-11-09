import {useEffect,useState} from 'react'
import {useQuery} from '@tanstack/react-query'
import {Alert,Button,Card,Checkbox,Descriptions,Form,Input,Typography} from 'antd'
import {sourceAccess,type SourceBinding} from './api'
export function SourceConnections(){
 const [error,setError]=useState(''),[busy,setBusy]=useState(false)
 const q=useQuery({queryKey:['linjian','source-connections'],queryFn:sourceAccess.connections})
 const verify=async()=>{setError('');setBusy(true);try{await sourceAccess.verify();await q.refetch()}catch(e){setError((e as Error).message)}finally{setBusy(false)}}
 return <Card title="WeKeeper 连接">
 <Alert type="info" title="WeKeeper 地址和调用方凭据由部署配置统一管理。此处只查看连接和验证凭据，secret 不在页面填写或显示。"/>
 {(error||q.error)&&<Alert type="error" title={error||q.error?.message}/>}
 {q.data?.configured?<>
 <Descriptions column={1} items={[
 {key:'origin',label:'WeKeeper 地址',children:q.data.origin},
 {key:'key',label:'调用方凭据 ID（key_id）',children:<Typography.Text copyable>{q.data.key_id}</Typography.Text>},
 {key:'secret',label:'签名密钥（secret）',children:'已配置'},
 ]}/>
 <Button type="primary" loading={busy} onClick={verify}>验证连接</Button>
 {!q.data.connection&&<Alert type="info" title="尚未验证连接，验证成功后可配置学校消息路由。"/>}
 </>:q.data&&<Alert type="warning" title="尚未配置 WeKeeper 连接，请由部署人员填写服务地址、key_id 和 secret。"/>}
 {q.data?.connection&&<Descriptions column={1} items={[{key:'checked',label:'最近验证时间',children:q.data.connection.checked_at}]}/>}
 </Card>
}
export function SourceBindings({school,readOnly=false}:{school:string;readOnly?:boolean}){
 const [error,setError]=useState(''),[busy,setBusy]=useState(false)
 const [form]=Form.useForm<Pick<SourceBinding,'route_id'|'active'>>()
 const q=useQuery({queryKey:['linjian','source-bindings',school],queryFn:({signal})=>sourceAccess.bindings(school,signal)})
 const current=q.data?.binding
 useEffect(()=>{form.resetFields();form.setFieldsValue(current??{active:true});setError('')},[form,current,school])
 const save=async()=>{setError('');setBusy(true);try{const values=await form.validateFields();await sourceAccess.bind(school,values,current?.revision??0);await q.refetch()}catch(e){setError((e as Error).message??'请核对路由配置')}finally{setBusy(false)}}
 return <Card title="学校消息路由">
 <Alert type="info" title="每所学校只维护一条消息路由；所有学校统一使用部署配置中的 WeKeeper，各校使用独立的 route_id。保存会更新本校路由。"/>
 {(error||q.error)&&<Alert type="error" title={error||q.error?.message}/>}
 {current&&<Descriptions column={1} items={[
 {key:'id',label:'绑定标识',children:current.id},
 {key:'revision',label:'生效修订',children:current.revision},
 {key:'checked',label:'路由检查',children:current.checked_at??'尚未校验'},
 {key:'active',label:'推送状态',children:current.active?'已启用':'已暂停，保留原事件'},
 ]}/>}
 {q.data?.backlog&&<Typography.Paragraph>待绑定 {q.data.backlog.waiting_binding??0} · 待配置 {q.data.backlog.waiting_configuration??0} · 暂停 {q.data.backlog.paused??0} · 阻塞 {q.data.backlog.blocked??0} · 撤回处理中 {q.data.backlog.withdrawal_jobs??0}</Typography.Paragraph>}
 {readOnly&&<Alert type="info" title="学校路由由平台管理员维护。"/>}
 <Form disabled={readOnly} form={form} layout="vertical" initialValues={{active:true}}>
 <Form.Item name="route_id" label="消息路由 ID（route_id）" rules={[{required:true,message:'请输入消息路由 ID'}]}><Input placeholder="WeKeeper 为本校分配的 route_id"/></Form.Item>
 <Form.Item name="active" valuePropName="checked"><Checkbox>启用消息推送</Checkbox></Form.Item>
 <Button type="primary" loading={busy} disabled={readOnly||q.isLoading||q.isFetching||!!q.error} onClick={save}>保存学校路由</Button>
 </Form>
 </Card>
}
