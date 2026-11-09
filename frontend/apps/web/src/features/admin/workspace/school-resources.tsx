import { useState } from 'react'
import { Button, Drawer } from 'antd'
import type { Row, School } from './api'
import { ResourceEditor } from './resource-editor'
import { RecordBody, RecordInformation, RecordList, defaultRecordColumns } from './record-list'

export function SchoolResources({ school, canEdit, refresh }: { school: School; canEdit: boolean; refresh: () => Promise<void> }) {
  const [editing, setEditing] = useState<Row>()
  const [open, setOpen] = useState(false)
  const editable = (record: Row) => canEdit && record.school_id === school.id
  function edit(record: Row) { setEditing(record); setOpen(true) }
  return <>
    {canEdit && <p>平台默认资源在本校只读；需要调整时新建本校同类配置并上传本校图片。</p>}
    <RecordList school={school.id} resource="resources" paginated={false} columns={defaultRecordColumns}
      actions={canEdit && <Button type="primary" onClick={() => { setEditing(undefined); setOpen(true) }}>新建展示资源</Button>}
      rowActions={(record) => editable(record) && <Button type="link" onClick={() => edit(record)}>编辑</Button>}
      detail={({ record, close }) => <>
        <RecordInformation record={record} />
        <section className="detail-section">
          <RecordBody record={record} />
          {editable(record) && <Button type="primary" onClick={() => { edit(record); close() }}>编辑</Button>}
        </section>
      </>}
    />
    {open && <Drawer open size="large" title={editing ? '编辑展示资源' : '新建展示资源'} onClose={() => { setOpen(false); setEditing(undefined) }}>
      <ResourceEditor key={editing?.id ?? 'new'} item={editing} school={school.id} onSaved={async () => {
        await refresh()
        setOpen(false)
        setEditing(undefined)
      }} />
    </Drawer>}
  </>
}
