import { useState } from 'react'
import { Form } from 'antd'
import { afterEach, expect, it, vi } from 'vitest'
import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react'
import { PostComments } from './post-comments'
import { workspace, type User } from './api'

vi.mock('./api', () => ({ workspace: { mutate: vi.fn() } }))
afterEach(() => {
  cleanup()
  vi.resetAllMocks()
})

function Host({ school = 's' }: { school?: string }) {
  const [form] = Form.useForm()
  const [error, setError] = useState('')
  return (
    <PostComments
      school={school}
      postId="post"
      user={{ id: 'u' } as User}
      comments={[{ id: 'comment', author_id: 'other', author_name: '同学', content: '正文' }]}
      form={form}
      busy={false}
      fetching={false}
      page={1}
      onPage={() => {}}
      error={error}
      run={async (action) => {
        try {
          await action()
        } catch (cause) {
          setError((cause as Error).message)
        }
      }}
    />
  )
}

it.each([
  ['approved', '评论已发布'],
  ['pending', '评论已提交，等待审核'],
  ['unknown', '评论已提交'],
])('confirms %s comments using the actual response state', async (audit_status, title) => {
  vi.mocked(workspace.mutate).mockResolvedValue({ audit_status })
  render(<Host />)
  fireEvent.change(screen.getByLabelText('评论内容'), { target: { value: '评论草稿' } })
  fireEvent.click(screen.getByRole('button', { name: '提交评论' }))
  await screen.findByText(title)
  expect(screen.getByLabelText('评论内容')).toHaveProperty('value', '')
})

it('preserves rejected comment drafts without a success message', async () => {
  vi.mocked(workspace.mutate).mockRejectedValueOnce(new Error('评论失败'))
  render(<Host />)
  fireEvent.change(screen.getByLabelText('评论内容'), { target: { value: '评论草稿' } })
  fireEvent.click(screen.getByRole('button', { name: '提交评论' }))
  await screen.findByText('评论失败')
  expect(screen.getByLabelText('评论内容')).toHaveProperty('value', '评论草稿')
  expect(screen.queryByText('评论已发布')).toBeNull()
})

it('confirms replies without clearing the independent comment draft', async () => {
  vi.mocked(workspace.mutate).mockResolvedValue({ audit_status: 'pending' })
  render(<Host />)
  fireEvent.change(screen.getByLabelText('评论内容'), { target: { value: '未提交的评论' } })
  fireEvent.click(screen.getByRole('button', { name: /^回\s*复$/ }))
  fireEvent.change(await screen.findByRole('textbox', { name: '回复内容' }), {
    target: { value: '回复草稿' },
  })
  fireEvent.click(screen.getByRole('button', { name: /^确\s*认$/ }))
  await screen.findByText('评论已提交，等待审核')
  expect(workspace.mutate).toHaveBeenCalledWith('s', 'posts/post/comments', {
    content: '回复草稿',
    parent_id: 'comment',
  })
  expect(screen.getByLabelText('评论内容')).toHaveProperty('value', '未提交的评论')
})

it.each(['cancel', 'empty'])('does not write or confirm a %s reply', async (action) => {
  render(<Host />)
  fireEvent.click(screen.getByRole('button', { name: /^回\s*复$/ }))
  await screen.findByRole('textbox', { name: '回复内容' })
  fireEvent.click(
    screen.getByRole('button', { name: action === 'cancel' ? /^取\s*消$/ : /^确\s*认$/ }),
  )
  await waitFor(() => expect(screen.queryByRole('textbox', { name: '回复内容' })).toBeNull())
  expect(workspace.mutate).not.toHaveBeenCalled()
})

it('does not submit a reply from a stale school dialog', async () => {
  const { rerender } = render(<Host />)
  fireEvent.click(screen.getByRole('button', { name: /^回\s*复$/ }))
  fireEvent.change(await screen.findByRole('textbox', { name: '回复内容' }), { target: { value: '过期回复' } })
  rerender(<Host school="other" />)
  fireEvent.click(screen.getByRole('button', { name: /^确\s*认$/ }))
  await waitFor(() => expect(screen.queryByRole('textbox', { name: '回复内容' })).toBeNull())
  expect(workspace.mutate).not.toHaveBeenCalled()
})
