import { RecordBody, RecordInformation, RecordList, defaultRecordColumns } from './record-list'

export function AuditLog({ school }: { school: string }) {
  return <RecordList school={school} resource="audit" columns={defaultRecordColumns} detail={({ record }) => <>
    <RecordInformation record={record} />
    <section className="detail-section"><RecordBody record={record} /></section>
  </>} />
}
