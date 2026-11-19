import { afterEach, expect, it, vi } from 'vitest'
import { cleanup, fireEvent, render, screen } from '@testing-library/react'
import { actionColumn, RowActions } from './row-actions'

afterEach(cleanup)

it('keeps the first actions visible and moves the rest into a labeled menu', async () => {
  const detail = vi.fn()
  const members = vi.fn()
  render(
    <RowActions
      label="阳光小学"
      actions={[
        { key: 'detail', label: '详情', onClick: detail },
        { key: 'edit', label: '编辑', onClick: vi.fn() },
        { key: 'members', label: '成员', onClick: members },
      ]}
    />,
  )
  fireEvent.click(screen.getByRole('button', { name: '详情 阳光小学' }))
  expect(detail).toHaveBeenCalledOnce()
  fireEvent.click(screen.getByRole('button', { name: '阳光小学 更多操作' }))
  fireEvent.click(await screen.findByText('成员'))
  expect(members).toHaveBeenCalledOnce()
})

it('creates a right-fixed action column with the requested width', () => {
  const column = actionColumn<{ id: string }>({
    width: 220,
    render: (row) => row.id,
  })
  expect(column).toMatchObject({
    title: '操作',
    fixed: 'right',
    width: 220,
  })
})
