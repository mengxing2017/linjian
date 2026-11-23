import { afterEach, beforeEach, expect, it, vi } from 'vitest'
import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { PostSeed } from './post-seed'
import { postSeed, type SeedBatch } from './api'
import { SchoolPermissionsContext } from './permissions-context'

vi.mock('./api', async (original) => ({
  ...(await original<typeof import('./api')>()),
  postSeed: {
    preview: vi.fn(),
    commit: vi.fn(),
    revoke: vi.fn(),
    list: vi.fn(),
    authors: vi.fn(),
    detail: vi.fn(),
    items: vi.fn(),
    avatar: vi.fn(),
    avatars: vi.fn(),
    authorAvatar: vi.fn(),
  },
}))
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
Object.defineProperty(URL, 'createObjectURL', { writable: true, value: () => 'blob:linjian-test' })
Object.defineProperty(URL, 'revokeObjectURL', { writable: true, value: () => {} })
const preview: SeedBatch = {
  id: 'batch',
  school_id: 'school',
  created_by: 'root',
  name: '运营批次',
  state: 'preview',
  revision: 1,
  preview_digest: 'a'.repeat(64),
  created_at: '2026-11-20T01:00:00Z',
  expires_at: '2026-11-20T01:30:00Z',
  committed_at: null,
  revoked_at: null,
  revoked_by: null,
  revoke_reason: null,
  row_labels: ['预览标题'],
  post_count: 0,
  interaction_count: 0,
  preview_payload: {
    authors: [
      {
        key: 'author-1',
        id: 'simulated',
        existing: false,
        nickname: '固定作者',
        avatar_id: 2,
        revision: null,
        avatar_url: null,
      },
    ],
    rows: [
      {
        author_key: 'author-1',
        title: '预览标题',
        content: '正文',
        tags: [],
        display_published_at: '2026-01-01T01:00:00Z',
      },
    ],
    errors: [],
    audit_mode: 'manual',
    avatar_version: 'linjian-student-avatars-v1',
  },
}
const committed: SeedBatch = { ...preview, state: 'committed', revision: 2, post_count: 1 }
beforeEach(() => {
  const page = { items: [], page: { number: 1, size: 20, has_more: false } }
  vi.mocked(postSeed.list).mockResolvedValue(page)
  vi.mocked(postSeed.authors).mockResolvedValue(page)
  vi.mocked(postSeed.items).mockResolvedValue(page)
  vi.mocked(postSeed.avatar).mockResolvedValue(new ArrayBuffer(0))
  vi.mocked(postSeed.avatars).mockResolvedValue({
    items: Array.from({ length: 30 }, (_, slot) => ({
      id: String(slot),
      slot,
      version: 'linjian-student-avatars-v1',
      url: `/api/v1/schools/school/post-seed-avatars/${slot}`,
    })),
    page: { number: 1, size: 30, has_more: false },
  })
  vi.mocked(postSeed.preview).mockResolvedValue(preview)
  vi.mocked(postSeed.detail).mockResolvedValue(committed)
  vi.mocked(postSeed.commit).mockResolvedValue(committed)
})
afterEach(() => {
  cleanup()
  vi.resetAllMocks()
})
function show(platform = true, effective: readonly string[] = []) {
  const client = new QueryClient({ defaultOptions: { queries: { retry: false } } })
  const saved = vi.fn().mockResolvedValue(undefined)
  render(
    <QueryClientProvider client={client}>
      <SchoolPermissionsContext.Provider value={{ platform, effective }}>
        <PostSeed school="school" saved={saved} onBlockingChange={() => {}} />
      </SchoolPermissionsContext.Provider>
    </QueryClientProvider>,
  )
  return saved
}
async function generate() {
  fireEvent.change(screen.getByLabelText('标题 1'), { target: { value: '校园生活' } })
  fireEvent.change(screen.getByLabelText('正文 1'), { target: { value: '正文' } })
  fireEvent.click(screen.getByRole('button', { name: '生成预览' }))
  await screen.findByText('固定作者')
}
it('submits only the fixed server preview with a stable idempotency key', async () => {
  const saved = show()
  await generate()
  expect(postSeed.preview).toHaveBeenCalledWith(
    'school',
    expect.objectContaining({
      rows: [expect.objectContaining({ title: '校园生活', author_key: 'author-1' })],
    }),
  )
  fireEvent.click(screen.getByRole('button', { name: '确认提交批次' }))
  await waitFor(() => expect(saved).toHaveBeenCalled())
  expect(postSeed.commit).toHaveBeenCalledWith('school', 'batch', {
    revision: 1,
    digest: 'a'.repeat(64),
    idempotency_key: expect.any(String),
  })
})
it('does not commit row errors and invalidates the preview after edits', async () => {
  vi.mocked(postSeed.preview).mockResolvedValue({
    ...preview,
    preview_payload: { ...preview.preview_payload, errors: [{ row: 1, message: '内容禁止发布' }] },
  })
  show()
  await generate()
  expect(screen.getByRole('button', { name: '确认提交批次' }).hasAttribute('disabled')).toBe(true)
  fireEvent.change(screen.getByLabelText('正文 1'), { target: { value: '已修改' } })
  expect(screen.queryByRole('button', { name: '确认提交批次' })).toBeNull()
  expect(postSeed.commit).not.toHaveBeenCalled()
})
it('keeps the original request when a commit response is uncertain', async () => {
  vi.mocked(postSeed.commit)
    .mockRejectedValueOnce(new Error('网络中断'))
    .mockResolvedValueOnce(committed)
  show()
  await generate()
  fireEvent.click(screen.getByRole('button', { name: '确认提交批次' }))
  await screen.findByText('网络中断')
  expect(screen.getByLabelText('正文 1').hasAttribute('disabled')).toBe(true)
  fireEvent.click(screen.getByRole('button', { name: '使用原请求确认结果' }))
  await waitFor(() => expect(postSeed.commit).toHaveBeenCalledTimes(2))
  expect(vi.mocked(postSeed.commit).mock.calls[0]).toEqual(vi.mocked(postSeed.commit).mock.calls[1])
})
it('does not load management data without an effective permission', () => {
  show(false)
  expect(screen.getByRole('button', { name: '生成预览' }).hasAttribute('disabled')).toBe(true)
  expect(postSeed.list).not.toHaveBeenCalled()
  expect(postSeed.authors).not.toHaveBeenCalled()
  expect(postSeed.avatars).not.toHaveBeenCalled()
})

it('loads migrated avatar choices from the catalog and submits the selected slot', async () => {
  show()
  fireEvent.click(await screen.findByRole('button', { name: '帖子 1 选择头像 30' }))
  expect(screen.queryByRole('button', { name: '帖子 1 选择头像 31' })).toBeNull()
  await generate()
  expect(postSeed.preview).toHaveBeenCalledWith(
    'school',
    expect.objectContaining({
      authors: [expect.objectContaining({ key: 'author-1', avatar_id: 29 })],
    }),
  )
})
it('allows an authorized school operator to publish simulation posts', async () => {
  show(false, ['content.post.read', 'content.simulation.manage'])
  await generate()
  expect(screen.getByRole('button', { name: '确认提交批次' }).hasAttribute('disabled')).toBe(false)
  expect(screen.getByText('发布仿真帖子')).toBeTruthy()
  expect(screen.getByLabelText('发布时间 1')).toBeTruthy()
  expect(screen.queryByText('上海展示时间')).toBeNull()
  expect(screen.queryByText('初始内容')).toBeNull()
})

it('hides pagination when author and batch lists have no further pages', async () => {
  show()
  const selector = screen.getByRole('combobox', { name: '作者 1' })
  fireEvent.mouseDown(selector)
  await screen.findByText('本校暂无已创建的模拟作者；新作者在批次成功提交后才可搜索复用。')
  await waitFor(() => expect(postSeed.authors).toHaveBeenCalled())
  expect(screen.queryByText('第 1 页')).toBeNull()
  expect(screen.queryByRole('button', { name: /上一页|下一页/ })).toBeNull()
  fireEvent.keyDown(selector, { key: 'Escape' })
  fireEvent.click(screen.getByRole('tab', { name: '批次管理' }))
  await waitFor(() => expect(postSeed.list).toHaveBeenCalled())
  expect(screen.queryByText('第 1 页')).toBeNull()
  expect(screen.queryByRole('button', { name: /上一页|下一页/ })).toBeNull()
})

it('keeps author pagination available to return from the last page', async () => {
  vi.mocked(postSeed.authors).mockImplementation(async (_school, page = 1) => ({
    items: [],
    page: { number: page, size: 20, has_more: page === 1 },
  }))
  show()
  fireEvent.mouseDown(screen.getByRole('combobox', { name: '作者 1' }))
  const next = await screen.findByRole('button', { name: '下一页模拟作者' })
  await waitFor(() => expect(next.hasAttribute('disabled')).toBe(false))
  fireEvent.click(next)
  await screen.findByText('第 2 页')
  const previous = screen.getByRole('button', { name: '上一页模拟作者' })
  await waitFor(() => expect(previous.hasAttribute('disabled')).toBe(false))
  expect(screen.getByRole('button', { name: '下一页模拟作者' }).hasAttribute('disabled')).toBe(true)
  fireEvent.click(previous)
  await screen.findByText('第 1 页')
  expect(postSeed.authors).toHaveBeenCalledWith('school', 2, '', expect.anything())
})

it('searches existing authors by nickname inside the selector and reuses the selected account', async () => {
  vi.mocked(postSeed.authors).mockImplementation(async (_school, page = 1, search = '') => ({
    items:
      search === '校园'
        ? [{ id: 'known-author', nickname: '校园微风', revision: 1, avatar_url: null }]
        : [],
    page: { number: page, size: 30, has_more: false },
  }))
  show()
  expect(screen.queryByLabelText('搜索已有模拟作者')).toBeNull()
  const selector = screen.getByRole('combobox', { name: '作者 1' })
  fireEvent.mouseDown(selector)
  fireEvent.change(selector, { target: { value: ' 校园 ' } })
  fireEvent.click(await screen.findByRole('option', { name: '校园微风' }))
  expect(postSeed.authors).toHaveBeenCalledWith('school', 1, '校园', expect.anything())
  expect(screen.queryByLabelText('昵称 1')).toBeNull()
  expect(selector.closest('.ant-select')?.textContent).toContain('校园微风')
  fireEvent.mouseDown(selector)
  fireEvent.change(selector, { target: { value: '不存在' } })
  await screen.findByText('未找到匹配的本校模拟作者，可修改昵称关键词或新建模拟作者。')
  expect(selector.closest('.ant-select')?.textContent).toContain('校园微风')
  fireEvent.keyDown(selector, { key: 'Escape' })
  await generate()
  expect(postSeed.preview).toHaveBeenCalledWith(
    'school',
    expect.objectContaining({
      authors: [expect.objectContaining({ key: 'author-1', existing_id: 'known-author' })],
    }),
  )
})

it('explains an unmatched author search and allows creating a new author instead', async () => {
  show()
  const selector = screen.getByRole('combobox', { name: '作者 1' })
  fireEvent.mouseDown(selector)
  fireEvent.change(selector, { target: { value: '不存在' } })
  await screen.findByText('未找到匹配的本校模拟作者，可修改昵称关键词或新建模拟作者。')
  fireEvent.click(screen.getByRole('option', { name: '新建模拟作者' }))
  expect(screen.getByLabelText('昵称 1')).toBeTruthy()
})

it('hides pagination for a single batch and its only page of posts', async () => {
  vi.mocked(postSeed.list).mockResolvedValue({
    items: [committed],
    page: { number: 1, size: 20, has_more: false },
  })
  show()
  fireEvent.click(screen.getByRole('tab', { name: '批次管理' }))
  fireEvent.click(await screen.findByRole('button', { name: /查\s*看/ }))
  await screen.findByRole('button', { name: '撤销整批' })
  expect(screen.queryByText('第 1 页')).toBeNull()
  expect(screen.queryByRole('button', { name: /上一页|下一页/ })).toBeNull()
})
