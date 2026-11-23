import { PermissionFixture, rolePermissions } from '../../../test/permissions'
import { afterEach, beforeEach, expect, it, vi } from 'vitest'
import {
  cleanup,
  fireEvent,
  render as renderView,
  screen,
  waitFor,
  within,
} from '@testing-library/react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { PostManagement } from './post-management'
import { postColumns } from './post-columns'
import { PostRowActions } from './post-actions'
import { SchoolPermissionsContext } from './permissions-context'
import { workspace, type PostManagementView } from './api'
vi.mock('./api', async original => ({
  ...await original<typeof import('./api')>(),
  workspace: {
    postManagement: {
      list: vi.fn(),
      pin: vi.fn(),
      status: vi.fn(),
      delete: vi.fn(),
      reorder: vi.fn(),
      detail: vi.fn(),
      recommendationPreview: vi
        .fn()
        .mockResolvedValue({ authorized: false, can_recommend: false, latest: null }),
      recommendations: vi.fn().mockResolvedValue({ items: [] }),
    },
    review: vi.fn(),
    mutate: vi.fn(),
  },
}))
const post: PostManagementView = {
  created_via: 'web',
  author_kind: 'ordinary',
  seed_batch_id: null,
  created_by: null,
  display_published_at: '2026-11-10T00:00:00Z',
  id: 'post-1',
  school_id: 'school',
  school_name: '甲学校',
  author_id: 'author',
  author_nickname: '公开昵称',
  title: '帖子甲', display_label: '帖子甲',
  content: '正文',
  images: [],
  tags: ['校园', '生活'],
  created_at: '2026-11-10T00:00:00Z',
  audit_status: 'approved',
  visibility: 'hidden',
  top_sort: 0,
  is_top: false,
  cover_image: null,
  source_summary: null,
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
    items: [post, { ...post, id: 'post-2', title: '帖子乙', display_label: '帖子乙' }],
    order_revision: 8,
    page: { number: 1, size: 30, has_more: false },
  })
})
afterEach(() => {
  cleanup()
  vi.resetAllMocks()
})
function mount(platformAdmin = false, effective = rolePermissions('admin')) {
  renderView(
    <SchoolPermissionsContext.Provider value={{ platform: false, effective }}>
      <QueryClientProvider
        client={new QueryClient({ defaultOptions: { queries: { retry: false } } })}
      >
        <PostManagement school="school" platformAdmin={platformAdmin} />
      </QueryClientProvider>
    </SchoolPermissionsContext.Provider>,
  )
}
it('shows content provenance columns in order and separates creation privileges from school management', async () => {
  mount()
  await screen.findAllByText('公开昵称')
  expect(screen.getAllByRole('columnheader').map((h) => h.textContent)).toEqual([
    '内容概览',
    '投稿来源',
    '模拟批次',
    '创建操作者',
    '发布时间',
    '学校',
    '昵称',
    '标签名称',
    '审核状态',
    '状态',
    '主图',
    '图集',
    '创建时间',
    '最近推送',
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
  expect(postColumns('school', vi.fn(), { toggle: vi.fn(), disabled: false }, {
    change: vi.fn(), disabled: false,
  })[0]).toMatchObject({
    title: '内容概览',
    fixed: 'left',
  })
  expect(row.getAllByRole('button').map((button) => button.getAttribute('aria-label'))).toContain(
    '帖子甲 更多操作',
  )
  const pin = row.getByRole('button', { name: '置顶 帖子甲' })
  const titleCell = row.getAllByRole('cell')[0]
  expect(titleCell.contains(pin)).toBe(true)
  expect(
    screen.getByText('帖子甲').compareDocumentPosition(pin) & Node.DOCUMENT_POSITION_FOLLOWING,
  ).toBeTruthy()
  expect(pin.getAttribute('aria-pressed')).toBe('false')
  expect(pin.textContent).toBe('')
  expect(row.getByRole('button', { name: '设置状态 帖子甲' })).toBeTruthy()
  fireEvent.click(row.getByRole('button', { name: '帖子甲 更多操作' }))
  await screen.findByRole('menuitem', { name: '删除' })
  expect(screen.queryByRole('menuitem', { name: /置顶/ })).toBeNull()
  for (const name of ['正常', '隐藏', '系统下架']) {
    expect(screen.queryByRole('menuitem', { name })).toBeNull()
  }
  fireEvent.click(pin)
  await waitFor(() =>
    expect(workspace.postManagement.pin).toHaveBeenCalledWith('school', 'post-1', 3, true),
  )
})
it('unpins from the filled title icon and disables it during submission', async () => {
  vi.mocked(workspace.postManagement.list).mockResolvedValue({
    items: [{ ...post, is_top: true }],
    order_revision: 8,
    page: { number: 1, size: 30, has_more: false },
  })
  let finish!: () => void
  vi.mocked(workspace.postManagement.pin).mockImplementationOnce(
    () => new Promise<PostManagementView>((resolve) => {
      finish = () => resolve(post)
    }),
  )
  mount()
  const pin = await screen.findByRole('button', { name: '取消置顶 帖子甲' })
  expect(pin.getAttribute('aria-pressed')).toBe('true')
  expect(pin.querySelector('[data-icon="pushpin"]')).toBeTruthy()
  fireEvent.click(pin)
  await waitFor(() =>
    expect(workspace.postManagement.pin).toHaveBeenCalledWith('school', 'post-1', 3, false),
  )
  expect(pin.hasAttribute('disabled')).toBe(true)
  expect(screen.getByRole('button', { name: '设置状态 帖子甲' }).hasAttribute('disabled')).toBe(true)
  fireEvent.click(pin)
  expect(workspace.postManagement.pin).toHaveBeenCalledTimes(1)
  finish()
  await waitFor(() => expect(pin.hasAttribute('disabled')).toBe(false))
})
it('disables the title pin icon without pin permission', async () => {
  mount(false, ['content.post.read'])
  const pin = await screen.findByRole('button', { name: '置顶 帖子甲' })
  expect(pin.hasAttribute('disabled')).toBe(true)
  const status = screen.getByRole('button', { name: '设置状态 帖子甲' })
  expect(status.hasAttribute('disabled')).toBe(true)
  fireEvent.click(status)
  expect(screen.queryByRole('menuitem', { name: '正常' })).toBeNull()
  fireEvent.click(pin)
  expect(workspace.postManagement.pin).not.toHaveBeenCalled()
})
it('changes visibility directly from the status column', async () => {
  mount()
  await screen.findByText('帖子甲')
  const status = screen.getByRole('button', { name: '设置状态 帖子甲' })
  expect(status.textContent).toContain('隐藏')
  const row = within(screen.getByText('帖子甲').closest('tr')!)
  const statusIndex = screen.getAllByRole('columnheader').findIndex((header) => header.textContent === '状态')
  expect(row.getAllByRole('cell')[statusIndex].contains(status)).toBe(true)
  fireEvent.click(status)
  const hidden = await screen.findByRole('menuitem', { name: '隐藏' })
  expect(hidden.getAttribute('aria-disabled')).toBe('true')
  fireEvent.click(hidden)
  expect(workspace.postManagement.status).not.toHaveBeenCalled()
  fireEvent.click(screen.getByRole('menuitem', { name: '正常' }))
  await waitFor(() =>
    expect(workspace.postManagement.status).toHaveBeenCalledWith('school', 'post-1', 3, 'visible'),
  )
})
it('still requires an offline reason when the action is opened from the status column', async () => {
  mount()
  await screen.findByText('帖子甲')
  fireEvent.click(screen.getByRole('button', { name: '设置状态 帖子甲' }))
  fireEvent.click(await screen.findByRole('menuitem', { name: '系统下架' }))
  const dialog = within(await screen.findByRole('dialog'))
  const confirm = dialog.getByRole('button', { name: /^(OK|确\s*定)$/ })
  expect(confirm.hasAttribute('disabled')).toBe(true)
  expect(workspace.postManagement.status).not.toHaveBeenCalled()
  fireEvent.change(dialog.getByLabelText('下架原因'), { target: { value: '违反学校规范' } })
  fireEvent.click(confirm)
  await waitFor(() =>
    expect(workspace.postManagement.status).toHaveBeenCalledWith(
      'school',
      'post-1',
      3,
      'offline',
      '违反学校规范',
    ),
  )
})
it.each(['visible', 'offline'] as const)('hides a %s post from the status column and refreshes its label', async (visibility) => {
  vi.mocked(workspace.postManagement.list).mockResolvedValue({
    items: [{ ...post, visibility }],
    order_revision: 8,
    page: { number: 1, size: 30, has_more: false },
  })
  vi.mocked(workspace.postManagement.status).mockImplementationOnce(async () => {
    vi.mocked(workspace.postManagement.list).mockResolvedValue({
      items: [post],
      order_revision: 9,
      page: { number: 1, size: 30, has_more: false },
    })
    return post
  })
  mount()
  const status = await screen.findByRole('button', { name: '设置状态 帖子甲' })
  expect(status.textContent).toContain(visibility === 'visible' ? '正常' : '系统下架')
  fireEvent.click(status)
  fireEvent.click(await screen.findByRole('menuitem', { name: '隐藏' }))
  await waitFor(() =>
    expect(workspace.postManagement.status).toHaveBeenCalledWith('school', 'post-1', 3, 'hidden'),
  )
  await waitFor(() => expect(status.textContent).toContain('隐藏'))
})
it.each(['确认', '取消'])('requires deletion confirmation from more: %s', async (choice) => {
  mount()
  await screen.findByText('帖子甲')
  fireEvent.click(screen.getByRole('button', { name: '帖子甲 更多操作' }))
  fireEvent.click(await screen.findByRole('menuitem', { name: '删除' }))
  const dialog = within(
    await screen.findByRole('dialog', {
      name: '确认删除“帖子甲”？帖子 ID：post-1。删除后无法恢复。',
    }),
  )
  expect(workspace.postManagement.delete).not.toHaveBeenCalled()
  fireEvent.click(dialog.getByRole('button', { name: choice === '确认' ? /确\s*认/ : /取\s*消/ }))
  await waitFor(() => expect(screen.queryByRole('dialog')).toBeNull())
  if (choice === '确认') {
    expect(workspace.postManagement.delete).toHaveBeenCalledWith('school', 'post-1', 3)
  } else {
    expect(workspace.postManagement.delete).not.toHaveBeenCalled()
  }
})
it('opens the existing push workflow from more instead of an inline button', async () => {
  const recommend = vi.fn()
  render(
    <PostRowActions
      school="school"
      row={post}
      disabled={false}
      review={vi.fn()}
      edit={vi.fn()}
      recommend={recommend}
      act={async (action) => {
        await action()
      }}
    />,
  )
  expect(screen.queryByRole('button', { name: /推\s*送/ })).toBeNull()
  fireEvent.click(screen.getByRole('button', { name: '帖子甲 更多操作' }))
  fireEvent.click(await screen.findByRole('menuitem', { name: '推送' }))
  expect(recommend).toHaveBeenCalledWith('post-1')
})
it.each([false, true])(
  'keeps menu actions guarded by permissions and busy state: %s',
  async (busy) => {
    const recommend = vi.fn()
    renderView(
      <SchoolPermissionsContext.Provider
        value={{
          platform: false,
          effective: busy ? rolePermissions('admin') : ['content.post.read'],
        }}
      >
        <PostRowActions
          school="school"
          row={post}
          disabled={busy}
          review={vi.fn()}
          edit={vi.fn()}
          recommend={recommend}
          act={async (action) => {
            await action()
          }}
        />
      </SchoolPermissionsContext.Provider>,
    )
    expect(screen.getByRole('button', { name: '编辑 帖子甲' }).hasAttribute('disabled')).toBe(true)
    fireEvent.click(screen.getByRole('button', { name: '帖子甲 更多操作' }))
    const remove = await screen.findByRole('menuitem', { name: '删除' })
    expect(screen.queryByRole('menuitem', { name: /置顶/ })).toBeNull()
    expect(screen.queryByRole('menuitem', { name: '正常' })).toBeNull()
    expect(screen.getByRole('menuitem', { name: '删除' }).getAttribute('aria-disabled')).toBe(
      'true',
    )
    fireEvent.click(remove)
    expect(workspace.postManagement.delete).not.toHaveBeenCalled()
    expect(workspace.postManagement.status).not.toHaveBeenCalled()
    expect(workspace.postManagement.pin).not.toHaveBeenCalled()
    expect(recommend).not.toHaveBeenCalled()
    if (!busy) expect(screen.queryByRole('menuitem', { name: '推送' })).toBeNull()
  },
)
it.each(['web', 'seed'] as const)(
  'keeps push notices and history out of %s post details',
  async (created_via) => {
    const detail = { ...post, created_via }
    vi.mocked(workspace.postManagement.list).mockResolvedValue({
      items: [detail],
      order_revision: 8,
      page: { number: 1, size: 30, has_more: false },
    })
    vi.mocked(workspace.review).mockResolvedValue(detail)
    mount()
    const row = within((await screen.findByText('帖子甲')).closest('tr')!)
    fireEvent.click(row.getByRole('button', { name: '详情 帖子甲' }))
    const drawer = within(await screen.findByRole('dialog'))
    await drawer.findByText('正文')
    expect(drawer.getByText('公开昵称')).toBeTruthy()
    expect(drawer.queryByText('仿真发帖仅站内展示，不允许来源推送。')).toBeNull()
    expect(
      drawer.queryByText('质量由学校管理员、运营人员人工判断。已受理不代表实际发送成功。'),
    ).toBeNull()
    expect(drawer.queryByRole('columnheader', { name: '发起时间' })).toBeNull()
    expect(drawer.queryByRole('columnheader', { name: '发起类型' })).toBeNull()
    expect(drawer.queryByText('第 1 页')).toBeNull()
    expect(drawer.queryByRole('button', { name: '刷新推送记录' })).toBeNull()
    expect(workspace.postManagement.recommendationPreview).not.toHaveBeenCalled()
    expect(workspace.postManagement.recommendations).not.toHaveBeenCalled()
  },
)
it('permutes the current page with keyboard controls and sends the original revisions and order token', async () => {
  mount(true)
  await screen.findByText('帖子甲')
  expect(screen.getByRole('button', { name: /添\s*加/ })).toBeTruthy()
  expect(screen.getByRole('button', { name: /导\s*入/ })).toBeTruthy()
  fireEvent.click(screen.getByRole('button', { name: '普通排序' }))
  await waitFor(() =>
    expect(screen.getByRole('button', { name: '设置状态 帖子甲' }).hasAttribute('disabled')).toBe(true),
  )
  await waitFor(() =>
    expect(screen.getByRole('button', { name: '置顶 帖子甲' }).hasAttribute('disabled')).toBe(true),
  )
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
  fireEvent.change(await screen.findByRole('textbox', { name: '标签名称' }), {
    target: { value: '生活' },
  })
  const createdFrom = screen.getByLabelText('创建起始时间')
  fireEvent.focus(createdFrom)
  fireEvent.change(createdFrom, { target: { value: '2026-11-10 08:00' } })
  fireEvent.keyDown(createdFrom, { key: 'Enter', keyCode: 13 })
  await waitFor(() =>
    expect(workspace.postManagement.list).toHaveBeenLastCalledWith(
      'school',
      expect.objectContaining({
        search: '校园',
        tag: '生活',
        created_from: new Date('2026-11-10T08:00').toISOString(),
        page: 1,
      }),
      expect.any(AbortSignal),
    ),
  )
  expect(screen.getByRole('button', { name: /筛选 \(2\)/ })).toBeTruthy()
  fireEvent.click(screen.getByRole('button', { name: '重置筛选' }))
  await waitFor(() =>
    expect(workspace.postManagement.list).toHaveBeenLastCalledWith(
      'school',
      expect.objectContaining({ search: '校园', tag: undefined, created_from: undefined, page: 1 }),
      expect.any(AbortSignal),
    ),
  )
  expect((screen.getByLabelText('创建起始时间') as HTMLInputElement).value).toBe('')
})

function render(ui: Parameters<typeof renderView>[0]) {
  return renderView(<PermissionFixture>{ui}</PermissionFixture>)
}
