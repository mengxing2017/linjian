import { Button, Dropdown, Space, type TableColumnsType } from 'antd'
import type { ReactNode } from 'react'

export interface RowAction {
  key: string
  label: string
  onClick?: () => void
  danger?: boolean
  disabled?: boolean
}

export function RowActions({
  label,
  actions,
  primaryCount = 2,
}: {
  label: string
  actions: RowAction[]
  primaryCount?: number
}) {
  const primary = actions.slice(0, primaryCount)
  const more = actions.slice(primaryCount)
  return (
    <Space size={4} wrap={false}>
      {primary.map((action) => (
        <Button
          key={action.key}
          type="link"
          danger={action.danger}
          disabled={action.disabled}
          aria-label={`${action.label} ${label}`}
          onClick={action.onClick}
        >
          {action.label}
        </Button>
      ))}
      {more.length > 0 && (
        <Dropdown
          trigger={['click']}
          menu={{
            items: more.map(({ key, label, danger, disabled }) => ({
              key,
              label,
              danger,
              disabled,
            })),
            onClick: ({ key }) => more.find((item) => item.key === key)?.onClick?.(),
          }}
        >
          <Button type="link" aria-label={`${label} 更多操作`}>
            更多
          </Button>
        </Dropdown>
      )}
    </Space>
  )
}

export function actionColumn<T>({
  width,
  render,
}: {
  width: number
  render: (row: T) => ReactNode
}): TableColumnsType<T>[number] {
  return {
    title: '操作',
    key: 'actions',
    fixed: 'right',
    width,
    render: (_, row) => render(row),
  }
}
