import { useState } from 'react'
import { Select } from 'antd'
import { workspace, type CaseManagementView } from './api'
import { RecordList } from './record-list'
import { CaseDetail } from './case-detail'
import { caseColumns, caseStates } from './case-columns'

export function Cases({ school, refresh }: { school: string; refresh: () => Promise<void> }) {
  const [status, setStatus] = useState('')
  return <RecordList<CaseManagementView> school={school} resource="cases" loader={workspace.cases} extraQuery={{kind:'report'}}
    columns={caseColumns} showCreatedAt={false} scrollWidth={1550}
    searchPlaceholder="搜索标题、举报原因或举报人" status={status} onStatusChange={setStatus}
    filters={change => <Select aria-label="处理状态" value={status} onChange={change} style={{ width: 150 }} options={[
      { value: '', label: '全部处理状态' }, ...Object.entries(caseStates).map(([value, label]) => ({ value, label })),
    ]} />}
    detail={props => <CaseDetail {...props} school={school} refresh={refresh} />} />
}
