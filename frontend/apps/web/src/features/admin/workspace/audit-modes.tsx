import { useState } from 'react'
import { useQuery, useQueryClient } from '@tanstack/react-query'
import { Alert, Button, Form, Select, Space } from 'antd'
import { workspace, type School } from './api'

const options = [
  { value: 'manual', label: '人工审核' },
  { value: 'auto', label: '自动审核（自动通过）' },
]
export function AuditModeFields({ disabled = false }: { disabled?: boolean }) {
  return (
    <>
      <Form.Item name={['config', 'post_audit_mode']} label="帖子审核模式">
        <Select aria-label="帖子审核模式" disabled={disabled} options={options} />
      </Form.Item>
      <Form.Item name={['config', 'comment_audit_mode']} label="评论审核模式">
        <Select aria-label="评论审核模式" disabled={disabled} options={options} />
      </Form.Item>
    </>
  )
}
export function AuditModes({ school, canEdit }: { school: School; canEdit: boolean }) {
  const [saved, setSaved] = useState(false)
  const q = useQuery({
    queryKey: ['linjian', 'audit-policy', school.id],
    queryFn: ({ signal }) => workspace.schoolProfiles.detail(school.id, signal),
    refetchOnWindowFocus: false,
    refetchOnReconnect: false,
  })
  return (
    <Space orientation="vertical" className="full-width">
      {saved && <Alert type="success" title="审核模式已保存" />}
      {q.error && <Alert type="error" title={q.error.message} />}
      {q.data && (
        <ModeForm
          key={`${q.data.id}:${q.data.revision}`}
          school={q.data}
          canEdit={canEdit}
          onSaved={() => setSaved(true)}
          onChange={() => setSaved(false)}
          reload={async () => {
            setSaved(false)
            await q.refetch()
          }}
        />
      )}
    </Space>
  )
}
function ModeForm({
  school,
  canEdit,
  reload,
  onSaved,
  onChange,
}: {
  school: School
  canEdit: boolean
  reload: () => Promise<void>
  onSaved: () => void
  onChange: () => void
}) {
  const cache = useQueryClient()
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState('')
  const [conflict, setConflict] = useState(false)
  const disabled = !canEdit || school.status !== 'normal' || busy || conflict
  return (
    <Form
      layout="inline"
      initialValues={{
        config: {
          post_audit_mode: school.config.post_audit_mode === 'auto' ? 'auto' : 'manual',
          comment_audit_mode: school.config.comment_audit_mode === 'auto' ? 'auto' : 'manual',
        },
      }}
      onValuesChange={onChange}
      onFinish={async (values: { config: School['config'] }) => {
        if (disabled) return
        setBusy(true)
        setError('')
        try {
          await workspace.schoolProfiles.update(school.id, {
            name: school.name,
            revision: school.revision,
            config: values.config,
          })
          await cache.invalidateQueries({ queryKey: ['schools'] })
          await cache.invalidateQueries({ queryKey: ['linjian', 'school-management'] })
          await cache.invalidateQueries({ queryKey: ['linjian', 'post-reviews', school.id] })
          await reload()
          onSaved()
        } catch (e) {
          setError((e as Error).message)
          if (e && typeof e === 'object' && 'status' in e && e.status === 409) setConflict(true)
        } finally {
          setBusy(false)
        }
      }}
    >
      <p className="full-width">
        {school.name}
        ：仅影响之后的新投稿及内容编辑，已有审核状态不变。自动审核在基础校验后直接通过并展示。
      </p>
      <AuditModeFields disabled={disabled} />
      {canEdit && (
        <Button htmlType="submit" type="primary" disabled={disabled} loading={busy}>
          保存审核模式
        </Button>
      )}
      {error && <Alert type="error" title={error} />}
      {conflict && (
        <Space>
          <Alert type="warning" title="学校配置已变更，选择已保留，请重新加载后核对。" />
          <Button onClick={() => void reload()}>重新加载审核模式</Button>
        </Space>
      )}
    </Form>
  )
}
