import { useState } from 'react'
import { useQuery } from '@tanstack/react-query'
import { Alert, Button, Space, Switch, Table } from 'antd'
import { serviceApi } from './api'
import { useSchoolPermission } from '../workspace/permissions-context'
export function SchoolMarketing({school}:{school:string}) {
  const can=useSchoolPermission(),[error,setError]=useState('')
  const query=useQuery({queryKey:['linjian','school-marketing-settings',school],queryFn:()=>serviceApi.marketingSettings(school)})
  return <section><h2>本校官方营销</h2><p>只启停平台已核验单元，不能修改官方单元或配置站外链接。数据为客户端生命周期，不是官方结算收入。</p>
    {(error||query.error)&&<Alert type="error" title={error||query.error?.message}/>}
    <Table rowKey="id" dataSource={query.data?.items} pagination={false} columns={[
      {title:'营销位',dataIndex:'name'},{title:'形态',dataIndex:'format'},{title:'平台状态',render:(_,row)=>row.available?'可用':'已停用'},
      {title:'本校启用',render:(_,row)=><Switch aria-label={`启用营销位：${row.name}`} checked={row.enabled} disabled={!can('operation.marketing.update')||!row.available} onChange={enabled=>{void serviceApi.marketingSetting(school,row.id,row.revision,enabled).then(()=>query.refetch()).catch(cause=>setError(cause instanceof Error?cause.message:'保存失败'))}}/>},
    ]}/><Space><Button onClick={()=>void query.refetch()}>刷新</Button></Space>
  </section>
}
