import { afterEach, beforeEach, expect, it, vi } from 'vitest'
import { cleanup, fireEvent, render, screen, waitFor, within } from '@testing-library/react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { PostManagement } from './post-management'
import { workspace, type PostManagementView } from './api'
vi.mock('./api', () => ({
  workspace: {
    postManagement: {
      list: vi.fn(),
      pin: vi.fn(),
      status: vi.fn(),
      delete: vi.fn(),
      reorder: vi.fn(),
      detail: vi.fn(),
    },
    review: vi.fn(),
    mutate: vi.fn(),
  },
}))
const post: PostManagementView = {
  id: 'post-1',
  school_id: 'school',
  school_name: '甲学校',
  author_id: 'author',
  author_nickname: '公开昵称',
  title: '帖子甲',
  content: '正文',
  images: [],
  tags: ['校园', '生活'],
  created_at: '2026-11-10T00:00:00Z',
  audit_status: 'approved',
  visibility: 'hidden',
  top_sort: 0,
  is_top: false,
  cover_image: null,
  revision: 3,
  content_revision: 1,
  review_digest: 'digest',
  decision_source: 'human',
  decided_at: null,
  moderator_id: null,
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
  vi.mocked(workspace.postManagement.list).mockResolvedValue({
    items: [post, { ...post, id: 'post-2', title: '帖子乙' }],
    order_revision: 8,
    page: { number: 1, size: 30, has_more: false },
  })
})
afterEach(() => {
  cleanup()
  vi.resetAllMocks()
})
function mount(platformAdmin = false) {
  render(
    <QueryClientProvider
      client={new QueryClient({ defaultOptions: { queries: { retry: false } } })}
    >
      <PostManagement school="school" platformAdmin={platformAdmin} />
    </QueryClientProvider>,
  )
}
it('shows the ten business columns in order and separates creation privileges from school management', async () => {
  mount()
  await screen.findAllByText('公开昵称')
  expect(screen.getAllByRole('columnheader').map((h) => h.textContent)).toEqual([
    '',
    '标题',
    '学校',
    '昵称',
    '标签名称',
    '审核状态',
    '置顶状态',
    '状态',
    '主图',
    '图集',
    '创建时间',
    '操作',
  ])
  expect(screen.getAllByText('校园、生活')).toHaveLength(2)
  expect(screen.queryByRole('button', { name: /添\s*加/ })).toBeNull()
  expect(screen.queryByRole('button', { name: /导\s*入/ })).toBeNull()
  expect(workspace.postManagement.list).toHaveBeenCalledWith(
    'school',
    { page: 1 },
    expect.any(AbortSignal),
  )
  const row = within(screen.getByText('帖子甲').closest('tr')!)
  fireEvent.click(row.getByRole('button', { name: /置\s*顶/ }))
  await waitFor(() =>
    expect(workspace.postManagement.pin).toHaveBeenCalledWith('school', 'post-1', 3, true),
  )
})
it('permutes the current page with keyboard controls and sends the original revisions and order token', async () => {
  mount(true)
  await screen.findByText('帖子甲')
  expect(screen.getByRole('button', { name: /添\s*加/ })).toBeTruthy()
  expect(screen.getByRole('button', { name: /导\s*入/ })).toBeTruthy()
  fireEvent.click(screen.getByRole('button', { name: '普通排序' }))
  await waitFor(() =>
    expect(workspace.postManagement.list).toHaveBeenLastCalledWith(
      'school',
      { page: 1, is_top: false },
      expect.any(AbortSignal),
    ),
  )
  await waitFor(() =>
    expect(screen.getByRole('button', { name: '下移 帖子甲' }).hasAttribute('disabled')).toBe(
      false,
    ),
  )
  fireEvent.click(screen.getByRole('button', { name: '下移 帖子甲' }))
  fireEvent.click(screen.getByRole('button', { name: '保存排序' }))
  await waitFor(() =>
    expect(workspace.postManagement.reorder).toHaveBeenCalledWith('school', {
      order_revision: 8,
      is_top: false,
      page: 1,
      page_size: 30,
      original: [
        { id: 'post-1', revision: 3 },
        { id: 'post-2', revision: 3 },
      ],
      ordered_ids: ['post-2', 'post-1'],
    }),
  )
})
it('restores sorting on conflict and blocks another submit until an explicit reload', async () => {
  vi.mocked(workspace.postManagement.reorder).mockRejectedValue(
    Object.assign(new Error('排序版本冲突'), { status: 409 }),
  )
  mount()
  await screen.findByText('帖子甲')
  fireEvent.click(screen.getByRole('button', { name: '普通排序' }))
  await waitFor(() =>
    expect(screen.getByRole('button', { name: '下移 帖子甲' }).hasAttribute('disabled')).toBe(
      false,
    ),
  )
  fireEvent.dragStart(screen.getByText('帖子甲').closest('tr')!)
  fireEvent.dragOver(screen.getByText('帖子乙').closest('tr')!)
  fireEvent.drop(screen.getByText('帖子乙').closest('tr')!)
  fireEvent.click(screen.getByRole('button', { name: '保存排序' }))
  await screen.findByText('排序未保存，已恢复原顺序。请重新加载列表后再排序。')
  expect(screen.getByRole('button', { name: '下移 帖子甲' }).hasAttribute('disabled')).toBe(true)
  expect(workspace.postManagement.reorder).toHaveBeenCalledTimes(1)
  fireEvent.click(screen.getByRole('button', { name: /刷\s*新/ }))
  await waitFor(() =>
    expect(screen.queryByText('排序未保存，已恢复原顺序。请重新加载列表后再排序。')).toBeNull(),
  )
})

it('keeps advanced filters collapsed and preserves search when resetting them', async () => {
  mount()
  await screen.findByText('帖子甲')
  expect(screen.queryByLabelText('创建起始时间')).toBeNull()
  fireEvent.change(screen.getByRole('searchbox', { name: '搜索' }), { target: { value: '校园' } })
  fireEvent.click(screen.getByRole('button', { name: /筛选/ }))
  fireEvent.change(await screen.findByRole('textbox', { name: '标签名称' }), { target: { value: '生活' } })
  fireEvent.change(screen.getByLabelText('创建起始时间'), { target: { value: '2026-11-10T08:00' } })
  await waitFor(() => expect(workspace.postManagement.list).toHaveBeenLastCalledWith(
    'school', expect.objectContaining({ search: '校园', tag: '生活', created_from: new Date('2026-11-10T08:00').toISOString(), page: 1 }), expect.any(AbortSignal),
  ))
  expect(screen.getByRole('button', { name: /筛选 \(2\)/ })).toBeTruthy()
  fireEvent.click(screen.getByRole('button', { name: '重置筛选' }))
  await waitFor(() => expect(workspace.postManagement.list).toHaveBeenLastCalledWith(
    'school', expect.objectContaining({ search: '校园', tag: undefined, created_from: undefined, page: 1 }), expect.any(AbortSignal),
  ))
  expect((screen.getByLabelText('创建起始时间') as HTMLInputElement).value).toBe('')
})
