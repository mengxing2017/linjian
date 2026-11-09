import { afterEach, expect, it, vi } from 'vitest'
import { cleanup, fireEvent, render, screen, waitFor, within } from '@testing-library/react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { CommentReviews } from './comment-reviews'
import { workspace } from './api'
import { schoolDefaults } from '../../../test/school'

vi.mock('./api', () => ({ workspace: { list: vi.fn(), mutate: vi.fn() } }))
vi.mock('../../../shared/use-dialogs', () => ({
  useDialogs: () => ({ contextHolder: null, prompt: async () => '已核对评论' }),
}))

afterEach(() => { cleanup(); vi.resetAllMocks() })

it('requires reloading a conflicted comment before submitting its new revision', async () => {
  const comment = { id: 'comment', content: '原评论', post_title: '所属帖子', author_name: '作者昵称', audit_status: 'pending', revision: 1 }
  vi.mocked(workspace.list).mockResolvedValue({ items: [comment] })
  vi.mocked(workspace.mutate).mockRejectedValueOnce(Object.assign(new Error('版本冲突'), { status: 409 }))
  const cache = new QueryClient({ defaultOptions: { queries: { retry: false } } })
  const refresh = vi.fn().mockResolvedValue(undefined)
  render(<QueryClientProvider client={cache}>
    <CommentReviews school={{ ...schoolDefaults, id: 'school', name: '学校', revision: 1, config: {} }} refresh={refresh} />
  </QueryClientProvider>)
  const row = within((await screen.findByText('原评论')).closest('tr')!)
  fireEvent.click(row.getByRole('button', { name: /详\s*情/ }))
  const drawer = await screen.findByRole('dialog')
  const detail = within(drawer)
  fireEvent.click(detail.getByRole('button', { name: '已拒绝' }))
  await detail.findByText('内容已变更，请重新加载并核对全文。')
  expect(detail.getByRole('button', { name: '已拒绝' })).toHaveProperty('disabled', true)
  expect(refresh).not.toHaveBeenCalled()

  vi.mocked(workspace.list).mockResolvedValue({ items: [{ ...comment, content: '新评论', revision: 2 }] })
  vi.mocked(workspace.mutate).mockResolvedValue({ id: comment.id })
  fireEvent.click(detail.getByRole('button', { name: '重新加载评论全文' }))
  await detail.findByText('新评论')
  await waitFor(() => expect(detail.getByRole('button', { name: '已拒绝' })).toHaveProperty('disabled', false))
  fireEvent.click(detail.getByRole('button', { name: '已拒绝' }))
  await waitFor(() => expect(workspace.mutate).toHaveBeenLastCalledWith('school', 'comments/comment/moderation-decisions', {
    revision: 2, decision: 'rejected', reason: '已核对评论',
  }))
  await waitFor(() => expect(drawer.isConnected).toBe(false))
  expect(refresh).toHaveBeenCalledOnce()
})
