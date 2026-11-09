import { Button, Popconfirm, Select, Space } from 'antd'
import { workspace, type PostManagementView, type PostVisibility } from './api'
import { postStatusLabels } from './post-columns'
export function PostRowActions({
  school,
  row,
  disabled,
  review,
  edit,
  offline,
  act,
}: {
  school: string
  row: PostManagementView
  disabled: boolean
  review: (id: string) => void
  edit: (id: string) => void
  offline: (row: PostManagementView) => void
  act: (action: () => Promise<unknown>) => Promise<void>
}) {
  return (
    <Space wrap>
      <Button disabled={disabled} onClick={() => review(row.id)}>
        {row.audit_status === 'pending' ? '审核详情' : '详情'}
      </Button>
      <Button disabled={disabled} onClick={() => edit(row.id)}>
        编辑
      </Button>
      <Button
        disabled={disabled}
        onClick={() =>
          void act(() => workspace.postManagement.pin(school, row.id, row.revision, !row.is_top))
        }
      >
        {row.is_top ? '取消置顶' : '置顶'}
      </Button>
      <Select
        aria-label={`设置状态 ${row.title}`}
        value={row.visibility}
        disabled={disabled}
        style={{ width: 120 }}
        options={Object.entries(postStatusLabels)
          .filter(([value]) => value !== 'deleted')
          .map(([value, label]) => ({ value, label }))}
        onChange={(value: PostVisibility) => {
          if (value === 'offline') offline(row)
          else void act(() => workspace.postManagement.status(school, row.id, row.revision, value))
        }}
      />
      <Popconfirm
        title="确认删除帖子？删除后无法恢复。"
        onConfirm={() => act(() => workspace.postManagement.delete(school, row.id, row.revision))}
      >
        <Button danger disabled={disabled}>
          删除
        </Button>
      </Popconfirm>
    </Space>
  )
}
