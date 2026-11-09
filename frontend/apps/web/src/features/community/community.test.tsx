import { schoolDefaults } from '../../test/school'
import type { ReactNode } from 'react'
import { afterEach, beforeEach, expect, it, vi } from 'vitest'
import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react'
import { MemoryRouter, Route, Routes } from 'react-router-dom'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import Community from './index'
import { product, workspace } from './api'

vi.mock('../school-context/api', async () => ({ schoolsApi: (await import('./api')).workspace }))
vi.mock('../identity', () => ({
  SessionGate: ({ children }: { children: (user: unknown) => ReactNode }) =>
    children({ id: 'u', display_name: '用户', bound_school_id: 's' }),
  label: (value: unknown) => String(value ?? ''),
}))
vi.mock('./api', () => ({
  identity: { profile: vi.fn() },
  product: { resources: { list: vi.fn().mockResolvedValue([]) } },
  workspace: {
    schools: vi.fn(),
    directory: vi.fn(),
    list: vi.fn(),
    mutate: vi.fn(),
    upload: vi.fn(),
    getPost: vi.fn(),
    comments: vi.fn(),
  },
}))
beforeEach(() => {
  vi.mocked(product.resources.list).mockResolvedValue([])
  vi.mocked(workspace.schools).mockResolvedValue([
    { id: 's', name: '测试学校', ...schoolDefaults, revision: 1, config: {} },
  ])
  vi.mocked(workspace.directory).mockResolvedValue({ items: [] })
  vi.mocked(workspace.list).mockResolvedValue({
    items: [],
    page: { number: 1, size: 20, has_more: false },
  })
  vi.mocked(workspace.mutate).mockResolvedValue({})
  vi.mocked(workspace.upload).mockResolvedValue({ url: '/image.png' })
  vi.mocked(workspace.comments).mockResolvedValue({ items: [] })
})
afterEach(() => {
  cleanup()
  vi.resetAllMocks()
})
function show(path = '/community') {
  const client = new QueryClient({ defaultOptions: { queries: { retry: false } } })
  render(
    <QueryClientProvider client={client}>
      <MemoryRouter initialEntries={[path]}>
        <Routes>
          <Route path="/community/:school/posts/:id" element={<Community />} />
          <Route path="/community" element={<Community />} />
        </Routes>
      </MemoryRouter>
    </QueryClientProvider>,
  )
}
it('uploads through the contract and submits managed form values with image URLs', async () => {
  show()
  fireEvent.click(await screen.findByRole('button', { name: /^发\s*布$/ }))
  fireEvent.change(screen.getByLabelText('标题'), { target: { value: '校园通知' } })
  fireEvent.change(screen.getByLabelText('正文'), { target: { value: '通知正文' } })
  fireEvent.change(screen.getByLabelText('标签（逗号分隔）'), { target: { value: '校园, 通知' } })
  const file = new File(['image'], 'photo.png', { type: 'image/png' })
  fireEvent.change(document.querySelector('input[type=file]')!, { target: { files: [file] } })
  await waitFor(() =>
    expect(workspace.upload).toHaveBeenCalledWith(
      's',
      expect.objectContaining({ name: 'photo.png' }),
    ),
  )
  await screen.findByAltText('上传预览')
  // The preview can render before upload invalidation releases the busy state.
  const submit = await screen.findByRole('button', { name: '提交审核' })
  await waitFor(() => expect(submit).toHaveProperty('disabled', false))
  fireEvent.click(submit)
  await waitFor(() =>
    expect(workspace.mutate).toHaveBeenCalledWith(
      's',
      'posts',
      expect.objectContaining({
        title: '校园通知',
        content: '通知正文',
        tags: ['校园', '通知'],
        images: ['/image.png'],
      }),
      'POST',
    ),
  )
})
it('validates feedback before sending and preserves an empty optional contact', async () => {
  show()
  fireEvent.click(await screen.findByRole('button', { name: /^反\s*馈$/ }))
  fireEvent.click(screen.getByRole('button', { name: '提交反馈' }))
  await screen.findByText('请填写意见反馈')
  expect(workspace.mutate).not.toHaveBeenCalled()
  fireEvent.change(screen.getByLabelText('意见反馈'), { target: { value: '建议内容' } })
  fireEvent.click(screen.getByRole('button', { name: '提交反馈' }))
  await waitFor(() =>
    expect(workspace.mutate).toHaveBeenCalledWith('s', 'cases', {
      kind: 'feedback',
      content: '建议内容',
      contact: '',
    }),
  )
})

it('preserves comment input across service switches and clears it after submission', async () => {
  vi.mocked(workspace.getPost).mockResolvedValue({
    id: 'post',
    school_id: 's',
    author_id: 'author',
    title: '帖子标题',
    content: '帖子正文',
    images: [],
    tags: [],
    audit_status: 'approved',
    visibility: 'visible',
    revision: 1,
  })
  show('/community/s/posts/post')
  fireEvent.change(await screen.findByLabelText('评论内容'), { target: { value: '保留的评论' } })
  fireEvent.click(screen.getByRole('button', { name: /^反\s*馈$/ }))
  expect(screen.queryByLabelText('评论内容')).toBeNull()
  fireEvent.click(screen.getByRole('button', { name: /^发\s*现$/ }))
  expect(await screen.findByLabelText('评论内容')).toHaveProperty('value', '保留的评论')
  fireEvent.click(screen.getByRole('button', { name: '提交评论' }))
  await waitFor(() =>
    expect(workspace.mutate).toHaveBeenCalledWith('s', 'posts/post/comments', {
      content: '保留的评论',
    }),
  )
  await waitFor(() => expect(screen.getByLabelText('评论内容')).toHaveProperty('value', ''))
})
