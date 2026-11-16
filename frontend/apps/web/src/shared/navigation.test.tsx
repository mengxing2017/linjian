import { useState } from 'react'
import { Pagination } from 'antd'
import { afterEach, expect, it, vi } from 'vitest'
import { cleanup, fireEvent, render, screen } from '@testing-library/react'
import { navigationPagination } from './navigation'
import { PaginationControls } from './pagination-controls'

afterEach(cleanup)

it('keeps page boundaries and busy state while changing pages through symbol buttons', () => {
  const change = vi.fn()
  const { rerender } = render(<PaginationControls page={1} more change={change} subject="帖子" />)
  const previous = screen.getByRole('button', { name: '上一页帖子' }) as HTMLButtonElement
  const next = screen.getByRole('button', { name: '下一页帖子' }) as HTMLButtonElement
  expect(previous.textContent).toBe('<')
  expect(previous.disabled).toBe(true)
  expect(next.textContent).toBe('>')
  fireEvent.click(next)
  expect(change).toHaveBeenCalledExactlyOnceWith(2)
  rerender(<PaginationControls page={2} more={false} change={change} subject="帖子" />)
  expect(next.disabled).toBe(true)
  fireEvent.click(previous)
  expect(change).toHaveBeenLastCalledWith(1)
  rerender(<PaginationControls page={2} more busy change={change} subject="帖子" />)
  expect(previous.disabled).toBe(true)
  expect(next.disabled).toBe(true)
})

it('preserves table pagination clicks, keyboard navigation and multi-page jumps', () => {
  const change = vi.fn()
  function Pages() {
    const [page, setPage] = useState(1)
    return <Pagination {...navigationPagination} current={page} total={200} showSizeChanger={false}
      onChange={next => { setPage(next); change(next) }} />
  }
  render(<Pages />)
  expect((screen.getByRole('button', { name: '上一页' }) as HTMLButtonElement).disabled).toBe(true)
  const next = screen.getByRole('button', { name: '下一页' })
  expect(next.textContent).toBe('>')
  expect(next.closest('li')?.hasAttribute('title')).toBe(false)
  fireEvent.click(next)
  expect(change).toHaveBeenLastCalledWith(2)
  fireEvent.keyDown(next.closest('li')!, { key: 'Enter', keyCode: 13 })
  expect(change).toHaveBeenLastCalledWith(3)
  fireEvent.click(screen.getByRole('button', { name: '向后跳页' }))
  expect(change).toHaveBeenLastCalledWith(8)
  fireEvent.click(screen.getByRole('button', { name: '向前跳页' }))
  expect(change).toHaveBeenLastCalledWith(3)
})
