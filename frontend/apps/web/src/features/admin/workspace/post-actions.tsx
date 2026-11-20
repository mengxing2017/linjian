import { useSchoolPermission } from './permissions-context'
import { workspace, type PostManagementView } from './api'
import { RowActions, type RowAction } from '../../../shared/row-actions'
import { useDialogs } from '../../../shared/use-dialogs'
export function PostRowActions({
  school,
  row,
  disabled,
  review,
  recommend,
  edit,
  act,
}: {
  school: string
  row: PostManagementView
  disabled: boolean
  recommend?: (id: string) => void
  review: (id: string) => void
  edit: (id: string) => void
  act: (action: () => Promise<unknown>) => Promise<void>
}) {
  const can = useSchoolPermission()
  const dialogs = useDialogs()
  const actions: RowAction[] = [
    {
      key: 'detail',
      label: row.audit_status === 'pending' ? '审核详情' : '详情',
      disabled,
      onClick: () => review(row.id),
    },
    {
      key: 'edit',
      label: '编辑',
      disabled: disabled || !can('content.post.edit'),
      onClick: () => edit(row.id),
    },
    ...(recommend && can('post.recommend')
      ? [
          {
            key: 'recommend',
            label: row.source_summary ? '重推' : '推送',
            disabled,
            onClick: () => recommend(row.id),
          },
        ]
      : []),
    {
      key: 'delete',
      label: '删除',
      danger: true,
      disabled: disabled || !can('content.post.delete'),
      onClick: async () => {
        if (await dialogs.confirm(`确认删除“${row.title}”？删除后无法恢复。`)) {
          await act(() => workspace.postManagement.delete(school, row.id, row.revision))
        }
      },
    },
  ]
  return (
    <>
      {dialogs.contextHolder}
      <RowActions label={row.title} actions={actions} />
    </>
  )
}
