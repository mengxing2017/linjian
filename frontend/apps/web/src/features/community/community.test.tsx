import { schoolDefaults } from '../../test/school'
import type { ReactNode } from 'react'
import { afterEach, beforeEach, expect, it, vi } from 'vitest'
import { act, cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react'
import { MemoryRouter, Route, Routes } from 'react-router-dom'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import Community from './index'
import { identity, product, workspace } from './api'

vi.mock('../school-context/api', async () => ({ schoolsApi: (await import('./api')).workspace }))
vi.mock('../identity', () => ({
  SessionGate: ({ children }: { children: (user: unknown) => ReactNode }) =>
    children({ id: 'u', display_name: '用户', bound_school_id: 's' }),
  label: (value: unknown) => String(value ?? ''),
}))
vi.mock('./api', async original => ({
  ...await original<typeof import('./api')>(),
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
  return client
}
it('submits body-only posts without opening the optional title field', async () => {
  show()
  fireEvent.click(await screen.findByRole('button', { name: /^发\s*布$/ }))
  expect(screen.queryByRole('textbox', { name: '标题（选填）' })).toBeNull()
  fireEvent.change(screen.getByLabelText('正文'), { target: { value: '随手写下的想法 👩‍💻' } })
  fireEvent.click(screen.getByRole('button', { name: '提交审核' }))
  await waitFor(() => expect(workspace.mutate).toHaveBeenCalledWith('s', 'posts', {
    title: '', content: '随手写下的想法 👩‍💻', images: [], tags: [], revision: undefined,
  }, 'POST'))
})

it.each(['feed', 'detail'])('renders body-only posts once with a usable entry in %s', async view => {
  const post = {
    id: 'post', school_id: 's', author_id: 'u', title: '', display_label: '正文内容',
    content: '正文内容', images: [], tags: [], audit_status: 'approved' as const,
    visibility: 'visible' as const, revision: 1,
    display_published_at: '2026-11-23T00:00:00Z', published_at_text: '2026-11-23 08:00',
  }
  vi.mocked(workspace.getPost).mockResolvedValue(post)
  vi.mocked(workspace.list).mockImplementation(async (_school, resource) => ({ items: resource === 'posts' ? [post] : [] }))
  show(view === 'detail' ? '/community/s/posts/post' : '/community')
  expect(await screen.findAllByText('正文内容')).toHaveLength(1)
  expect(screen.queryByRole('heading', { name: '校园动态' })).toBeNull()
  if (view === 'feed') expect(screen.getByRole('link', { name: '查看全文：正文内容' }).getAttribute('href')).toBe('/community/s/posts/post')
})
it.each(['feed', 'mine', 'detail'])(
  'shows only server-provided publication time in %s',
  async (view) => {
    const post = {
      id: 'post',
      school_id: 's',
      author_id: 'u',
      title: '我的帖子', display_label: '我的帖子',
      content: '正文',
      images: [],
      tags: [],
      audit_status: 'approved' as const,
      visibility: 'visible' as const,
      revision: 1,
      display_published_at: '2026-11-19T17:30:00Z',
      published_at_text: '2026-11-20 01:30',
      created_at: '2026-11-20T02:00:00Z',
    }
    vi.mocked(workspace.getPost).mockResolvedValue(post)
    vi.mocked(workspace.list).mockImplementation(async (_school, resource) => ({
      items: resource === 'posts' ? [post] : [],
      page: { number: 1, size: 20, has_more: false },
    }))
    show(view === 'detail' ? '/community/s/posts/post' : '/community')
    if (view === 'mine') fireEvent.click(await screen.findByRole('button', { name: '我的发布' }))
    const published = await screen.findByText(post.published_at_text)
    expect(published.getAttribute('datetime')).toBe(post.display_published_at)
    expect(screen.getByText('发布时间：')).toBeTruthy()
    expect(screen.queryByText(/创建时间/)).toBeNull()
    expect(screen.queryByText(post.created_at)).toBeNull()
  },
)
it('uploads through the contract and submits managed form values with image URLs', async () => {
  show()
  fireEvent.click(await screen.findByRole('button', { name: /^发\s*布$/ }))
  fireEvent.click(screen.getByRole('button', { name: '添加标题（选填）' }))
  fireEvent.change(screen.getByLabelText('标题（选填）'), { target: { value: '校园通知' } })
  fireEvent.change(screen.getByLabelText('正文'), { target: { value: '通知正文' } })
  const tags = screen.getByLabelText('标签')
  fireEvent.change(tags, { target: { value: '校园,通知' } })
  fireEvent.keyDown(tags, { key: 'Enter', code: 'Enter', keyCode: 13 })
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
  await screen.findByText('反馈已提交')
  expect(screen.queryByLabelText('意见反馈')).toBeNull()
  fireEvent.click(screen.getByRole('button', { name: '继续反馈' }))
  expect(screen.getByLabelText('意见反馈')).toHaveProperty('value', '')
  expect(workspace.mutate).toHaveBeenCalledTimes(1)
})

it('keeps feedback drafts on failure and offers a return only after submission succeeds', async () => {
  vi.mocked(workspace.mutate).mockRejectedValueOnce(new Error('提交失败'))
  show()
  fireEvent.click(await screen.findByRole('button', { name: /^反\s*馈$/ }))
  fireEvent.change(screen.getByLabelText('意见反馈'), { target: { value: '反馈草稿' } })
  fireEvent.change(screen.getByLabelText('联系方式'), { target: { value: '联系信息' } })
  fireEvent.click(screen.getByRole('button', { name: '提交反馈' }))
  await waitFor(() => expect(screen.getAllByText('提交失败').length).toBeGreaterThan(0))
  expect(screen.getByLabelText('意见反馈')).toHaveProperty('value', '反馈草稿')
  expect(screen.getByLabelText('联系方式')).toHaveProperty('value', '联系信息')
  expect(screen.queryByText('反馈已提交')).toBeNull()
  vi.mocked(workspace.mutate).mockResolvedValueOnce({})
  fireEvent.click(screen.getByRole('button', { name: '提交反馈' }))
  await screen.findByText('反馈已提交')
  await waitFor(() => expect(screen.getByRole('button', { name: '返回社区' })).toHaveProperty('disabled', false))
  fireEvent.click(screen.getByRole('button', { name: '返回社区' }))
  expect(screen.queryByText('反馈已提交')).toBeNull()
  expect(screen.getByRole('button', { name: /^发\s*现$/ }).getAttribute('aria-current')).toBe('page')
})

it('keeps feedback completion when refresh fails and retries reads without a second write', async () => {
  const client = show()
  fireEvent.click(await screen.findByRole('button', { name: /^反\s*馈$/ }))
  vi.spyOn(client, 'invalidateQueries').mockRejectedValueOnce(new Error('读取失败'))
  fireEvent.change(screen.getByLabelText('意见反馈'), { target: { value: '已提交的反馈' } })
  fireEvent.click(screen.getByRole('button', { name: '提交反馈' }))
  await screen.findByText('反馈已提交')
  await waitFor(() => expect(screen.getAllByText('反馈已提交，列表刷新失败，请刷新查看').length).toBeGreaterThan(0))
  fireEvent.click(screen.getByRole('button', { name: '重试刷新' }))
  await waitFor(() => expect(screen.queryByRole('button', { name: '重试刷新' })).toBeNull())
  expect(workspace.mutate).toHaveBeenCalledTimes(1)
})

it('returns from detail feedback to the current school community list', async () => {
  vi.mocked(workspace.getPost).mockResolvedValue({ id: 'post', school_id: 's', author_id: 'other', title: '详情', display_label: '详情', content: '正文', images: [], tags: [], audit_status: 'approved', visibility: 'visible', revision: 1, display_published_at: '2026-11-23T00:00:00Z', published_at_text: '2026-11-23 08:00' })
  show('/community/s/posts/post')
  fireEvent.click(await screen.findByRole('button', { name: /^反\s*馈$/ }))
  fireEvent.change(screen.getByLabelText('意见反馈'), { target: { value: '详情反馈' } })
  fireEvent.click(screen.getByRole('button', { name: '提交反馈' }))
  await screen.findByText('反馈已提交')
  await waitFor(() => expect(screen.getByRole('button', { name: '返回社区' })).toHaveProperty('disabled', false))
  fireEvent.click(screen.getByRole('button', { name: '返回社区' }))
  await screen.findByRole('button', { name: '上一页' })
  expect(screen.queryByLabelText('评论内容')).toBeNull()
  expect(screen.queryByText('反馈已提交')).toBeNull()
})

it('confirms nickname writes without leaving the profile page', async () => {
  vi.mocked(identity.profile).mockResolvedValue({} as Awaited<ReturnType<typeof identity.profile>>)
  show()
  fireEvent.click(await screen.findByRole('button', { name: '个人信息' }))
  fireEvent.change(screen.getByLabelText('昵称'), { target: { value: '新昵称' } })
  fireEvent.click(screen.getByRole('button', { name: '更新昵称' }))
  await screen.findByText('昵称已更新')
  expect(identity.profile).toHaveBeenCalledWith('新昵称')
  expect(screen.getByLabelText('昵称')).toHaveProperty('value', '新昵称')
})

it('keeps nickname input and never confirms a rejected save', async () => {
  vi.mocked(identity.profile).mockRejectedValueOnce(new Error('昵称保存失败'))
  show()
  fireEvent.click(await screen.findByRole('button', { name: '个人信息' }))
  fireEvent.change(screen.getByLabelText('昵称'), { target: { value: '保留昵称' } })
  fireEvent.click(screen.getByRole('button', { name: '更新昵称' }))
  await waitFor(() => expect(screen.getAllByText('昵称保存失败').length).toBeGreaterThan(0))
  expect(screen.getByLabelText('昵称')).toHaveProperty('value', '保留昵称')
  expect(screen.queryByText('昵称已更新')).toBeNull()
})

it('keeps nickname write confirmation when session refresh fails and retries only reads', async () => {
  vi.mocked(identity.profile).mockResolvedValue({} as Awaited<ReturnType<typeof identity.profile>>)
  const client = show()
  fireEvent.click(await screen.findByRole('button', { name: '个人信息' }))
  vi.spyOn(client, 'invalidateQueries').mockRejectedValueOnce(new Error('会话读取失败'))
  fireEvent.change(screen.getByLabelText('昵称'), { target: { value: '已保存昵称' } })
  fireEvent.click(screen.getByRole('button', { name: '更新昵称' }))
  await screen.findByText('昵称已更新')
  await waitFor(() => expect(screen.getAllByText('昵称已更新，显示信息刷新失败，请刷新查看').length).toBeGreaterThan(0))
  fireEvent.click(screen.getByRole('button', { name: '重试刷新' }))
  await waitFor(() => expect(screen.queryByRole('button', { name: '重试刷新' })).toBeNull())
  expect(identity.profile).toHaveBeenCalledTimes(1)
})

it('summarizes partial image uploads and retries only the failed image', async () => {
  vi.mocked(workspace.upload).mockResolvedValueOnce({ url: '/first.png' }).mockRejectedValueOnce(new Error('上传失败')).mockResolvedValueOnce({ url: '/retried.png' })
  show()
  fireEvent.click(await screen.findByRole('button', { name: /^发\s*布$/ }))
  const files = ['first.png', 'retry.png'].map(name => new File(['image'], name, { type: 'image/png' }))
  fireEvent.change(document.querySelector('input[type=file]')!, { target: { files } })
  await screen.findByText('本次已上传 1 张，失败 1 张，提交帖子后生效')
  await waitFor(() => expect(screen.getByRole('button', { name: '重试此图片' })).toHaveProperty('disabled', false))
  fireEvent.click(screen.getByRole('button', { name: '重试此图片' }))
  await screen.findByText('本次已上传 1 张，提交帖子后生效')
  expect(screen.getAllByAltText('上传预览')).toHaveLength(2)
  expect(workspace.upload).toHaveBeenCalledTimes(3)
  expect(vi.mocked(workspace.upload).mock.calls[2]?.[1]).toBe(files[1])
  expect(workspace.mutate).not.toHaveBeenCalled()
})

it('ignores an upload result after leaving the editor', async () => {
  let resolve!: (value: { url: string }) => void
  vi.mocked(workspace.upload).mockReturnValueOnce(new Promise(finish => { resolve = finish }))
  show()
  fireEvent.click(await screen.findByRole('button', { name: /^发\s*布$/ }))
  fireEvent.change(document.querySelector('input[type=file]')!, { target: { files: [new File(['image'], 'late.png', { type: 'image/png' })] } })
  await waitFor(() => expect(workspace.upload).toHaveBeenCalledTimes(1))
  fireEvent.click(screen.getByRole('button', { name: /^发\s*现$/ }))
  await act(async () => { resolve({ url: '/late.png' }) })
  fireEvent.click(screen.getByRole('button', { name: /^发\s*布$/ }))
  expect(screen.queryByAltText('上传预览')).toBeNull()
  expect(screen.queryByText('本次已上传 1 张，提交帖子后生效')).toBeNull()
})

it('preserves comment input across service switches and clears it after submission', async () => {
  vi.mocked(workspace.getPost).mockResolvedValue({
    id: 'post',
    display_published_at: '2026-11-10T00:00:00Z',
    published_at_text: '2026-11-10 08:00',
    school_id: 's',
    author_id: 'author',
    title: '帖子标题', display_label: '帖子标题',
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
