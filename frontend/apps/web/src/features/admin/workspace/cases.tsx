import { workspace } from './api'
import { Editor } from './editor'
import { RecordBody, RecordInformation, RecordList, defaultRecordColumns } from './record-list'

export function Cases({ school, refresh }: { school: string; refresh: () => Promise<void> }) {
  return <RecordList school={school} resource="cases" columns={defaultRecordColumns} detail={({ record }) => <>
    <RecordInformation record={record} />
    <section className="detail-section">
      <RecordBody record={record} />
      {record.state === 'open' && <Editor fields={[
        { name: 'resolution', title: '处理说明' },
        { name: 'state', title: '处理结果', options: [{ value: 'resolved', label: '已处理' }, { value: 'dismissed', label: '驳回' }] },
      ]} onSave={async values => {
        await workspace.mutate(school, `cases/${record.id}`, { ...values, revision: record.revision }, 'PUT')
        await refresh()
      }} />}
    </section>
  </>} />
}
