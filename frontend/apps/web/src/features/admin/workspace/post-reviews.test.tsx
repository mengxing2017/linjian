import { afterEach, beforeEach, expect, it, vi } from 'vitest'
import { cleanup, fireEvent, render, screen, waitFor, within } from '@testing-library/react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { PostReviews } from './post-reviews'
import { workspace, type PostReview } from './api'
vi.mock('./api', () => ({
  workspace: { reviews: vi.fn(), review: vi.fn(), reviewImage: vi.fn(), mutate: vi.fn() },
}))
const post: PostReview = {
  id: 'post-1',
  school_id: 'school-1',
  school_name: '校园甲',
  author_id: 'author-1',
  author_nickname: '小程序昵称',
  title: '完整标题',
  content: '原始正文\n第二行',
  created_at: '2026-11-09T00:00:00Z',
  images: [],
  tags: [],
  audit_status: 'pending',
  visibility: 'visible',
  top_sort: 0,
  revision: 3,
  content_revision: 2,
  review_digest: 'full-review-digest',
  decision_source: null, decided_at: null, moderator_id: null,
}
class ResizeObserver {
  observe() {}
  unobserve() {}
  disconnect() {}
}
vi.stubGlobal('ResizeObserver', ResizeObserver)
Object.defineProperty(window, 'matchMedia', {
  writable: true,
  value: () => ({
    matches: false,
    addListener() {},
    removeListener() {},
    addEventListener() {},
    removeEventListener() {},
  }),
})
beforeEach(() => {
  vi.mocked(workspace.reviews).mockResolvedValue({
    items: [post],
    page: { number: 1, size: 30, has_more: false },
  })
  vi.mocked(workspace.review).mockResolvedValue(post)
  vi.mocked(workspace.mutate).mockResolvedValue({ id: post.id })
})
afterEach(() => {
  cleanup()
  vi.resetAllMocks()
})
function mount() {
  return render(
    <QueryClientProvider
      client={new QueryClient({ defaultOptions: { queries: { retry: false } } })}
    >
      <PostReviews school="school-1" />
    </QueryClientProvider>,
  )
}
async function open() {
  fireEvent.click(await screen.findByRole('button', { name: /详\s*情/ }))
  const detail = within(await screen.findByRole('dialog'))
  await detail.findByText('原始正文 第二行', { exact: false })
  return detail
}
it('shows the bounded preview on hover and requires opening full content before any approval action', async () => {
  const ending = '\n<script>末尾原文</script>'
  const content = '文'.repeat(20000 - ending.length) + ending
  const full = { ...post, content }
  vi.mocked(workspace.reviews).mockResolvedValue({
    items: [full],
    page: { number: 1, size: 30, has_more: false },
  })
  vi.mocked(workspace.review).mockResolvedValue(full)
  mount()
  await screen.findByText('校园甲')
  for (const name of ['标题', '内容摘要', '学校', '用户昵称', '创建时间'])
    expect(screen.getByRole('columnheader', { name })).toBeTruthy()
  expect(screen.getByText('小程序昵称')).toBeTruthy()
  const preview = screen.getByText('文'.repeat(119) + '…')
  fireEvent.mouseEnter(preview)
  const tooltip = await screen.findByRole('tooltip')
  expect(tooltip.textContent).toBe('文'.repeat(119) + '…')
  fireEvent.mouseLeave(preview)
  expect(screen.queryByText(/末尾原文/)).toBeNull()
  expect(screen.queryByRole('button', { name: '通过审核' })).toBeNull()
  expect(screen.queryByRole('button', { name: /拒\s*绝/ })).toBeNull()
  expect(workspace.review).not.toHaveBeenCalled()
  expect(workspace.reviewImage).not.toHaveBeenCalled()
  fireEvent.click(screen.getByRole('button', { name: /详\s*情/ }))
  const detail = within(await screen.findByRole('dialog'))
  const paragraph = await detail.findByText(
    (_, element) => element?.tagName === 'P' && element.textContent === content,
  )
  expect(paragraph.textContent).toBe(content)
  expect(paragraph.querySelector('script')).toBeNull()
  expect(detail.getByRole('button', { name: '通过审核' })).toBeTruthy()
  expect(screen.queryByText('来源摘要预览')).toBeNull()
  expect(workspace.mutate).not.toHaveBeenCalled()
})
it('reads the latest full review and submits its revision and fingerprint', async () => {
  vi.mocked(workspace.review).mockResolvedValue({
    ...post,
    revision: 4,
    review_digest: 'latest-full-digest',
  })
  mount()
  const detail = await open()
  fireEvent.change(detail.getByRole('textbox', { name: '审核说明' }), {
    target: { value: '已核对全文' },
  })
  fireEvent.click(detail.getByRole('button', { name: '通过审核' }))
  await waitFor(() =>
    expect(workspace.mutate).toHaveBeenCalledWith('school-1', 'posts/post-1/moderation-decisions', {
      revision: 4,
      decision: 'approved',
      reason: '已核对全文',
      review_digest: 'latest-full-digest',
    }),
  )
})
it('retains the reason and requires an explicit full reload after a conflict', async () => {
  vi.mocked(workspace.mutate).mockRejectedValueOnce(
    Object.assign(new Error('版本冲突'), { status: 409 }),
  )
  mount()
  const detail = await open()
  fireEvent.change(detail.getByRole('textbox', { name: '审核说明' }), {
    target: { value: '保留审核说明' },
  })
  fireEvent.click(detail.getByRole('button', { name: '通过审核' }))
  await detail.findByText(/内容已变更/)
  expect(detail.getByRole('textbox', { name: '审核说明' })).toHaveProperty('value', '保留审核说明')
  expect(detail.getByRole('button', { name: '通过审核' })).toHaveProperty('disabled', true)
  expect(workspace.review).toHaveBeenCalledTimes(1)
  vi.mocked(workspace.review).mockResolvedValue({
    ...post,
    content: '重新读取的完整正文',
    revision: 4,
    review_digest: 'new-digest',
  })
  fireEvent.click(detail.getByRole('button', { name: '重新加载全文' }))
  await detail.findByText('重新读取的完整正文')
  fireEvent.click(detail.getByRole('button', { name: '通过审核' }))
  await waitFor(() =>
    expect(workspace.mutate).toHaveBeenLastCalledWith(
      'school-1',
      'posts/post-1/moderation-decisions',
      {
        revision: 4,
        decision: 'approved',
        reason: '保留审核说明',
        review_digest: 'new-digest',
      },
    ),
  )
})
it('blocks approval when an attachment cannot be read, while allowing rejection', async () => {
  vi.mocked(workspace.review).mockResolvedValue({
    ...post,
    images: ['/api/v1/schools/school-1/media/60000000-0000-0000-0000-000000000003'],
  })
  vi.mocked(workspace.reviewImage).mockRejectedValue(new Error('图片不可读取'))
  mount()
  const detail = await open()
  await detail.findByText(/附件 1 读取失败/)
  expect(detail.getByRole('button', { name: '通过审核' })).toHaveProperty('disabled', true)
  fireEvent.click(detail.getByRole('button', { name: /拒\s*绝/ }))
  await waitFor(() =>
    expect(workspace.mutate).toHaveBeenCalledWith('school-1', 'posts/post-1/moderation-decisions', {
      revision: 3,
      decision: 'rejected',
      reason: '',
    }),
  )
})
it('allows approval after an authenticated attachment retry is decoded successfully', async () => {
  Object.defineProperty(URL, 'createObjectURL', {
    configurable: true,
    value: vi.fn(() => 'blob:review-image'),
  })
  Object.defineProperty(URL, 'revokeObjectURL', { configurable: true, value: vi.fn() })
  vi.mocked(workspace.review).mockResolvedValue({
    ...post,
    images: ['/api/v1/schools/school-1/media/60000000-0000-0000-0000-000000000003'],
  })
  vi.mocked(workspace.reviewImage)
    .mockRejectedValueOnce(new Error('暂时失败'))
    .mockResolvedValue(new ArrayBuffer(4))
  mount()
  const detail = await open()
  await detail.findByText(/附件 1 读取失败/)
  fireEvent.click(detail.getByRole('button', { name: '重试附件 1' }))
  const image = await detail.findByAltText('帖子附件 1')
  expect(detail.getByRole('button', { name: '通过审核' })).toHaveProperty('disabled', true)
  fireEvent.load(image)
  await waitFor(() =>
    expect(detail.getByRole('button', { name: '通过审核' })).toHaveProperty('disabled', false),
  )
  expect(workspace.reviewImage).toHaveBeenCalledWith(
    'school-1',
    '60000000-0000-0000-0000-000000000003',
    expect.any(AbortSignal),
  )
})
it('cannot approve a cached review when reopening fails to read the latest full content', async () => {
  mount()
  const first = await open()
  fireEvent.click(first.getByRole('button', { name: 'Close' }))
  vi.mocked(workspace.review).mockRejectedValue(new Error('最新详情不可读取'))
  const second = await open()
  await second.findByText('最新详情不可读取')
  expect(second.getByText('原始正文 第二行', { exact: false })).toBeTruthy()
  expect(second.getByRole('button', { name: '通过审核' })).toHaveProperty('disabled', true)
  expect(workspace.mutate).not.toHaveBeenCalled()
})
