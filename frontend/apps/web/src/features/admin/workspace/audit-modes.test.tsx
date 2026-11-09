import { afterEach, beforeEach, expect, it, vi } from 'vitest'
import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { AuditModes } from './audit-modes'
import { workspace, type School } from './api'
vi.mock('./api', () => ({ workspace: { schoolProfiles: { detail: vi.fn(), update: vi.fn() } } }))
const school: School = {
  id: 'school-1',
  name: '学校甲',
  short_name: '', province: '', city: '', area: '', latitude: null, longitude: null,
  logo_asset_id: null, header_bg_asset_id: null, logo_url: null, header_bg_image_url: null,
  theme_primary: '#6A5AF9', theme_secondary: '#8F6BFF', theme_text_color: '#FFFFFF',
  status: 'normal',
  revision: 8,
  config: { post_audit_mode: 'manual', comment_audit_mode: 'manual', custom: 'keep' },
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
  vi.mocked(workspace.schoolProfiles.detail).mockResolvedValue(school)
  vi.mocked(workspace.schoolProfiles.update).mockResolvedValue({ ...school, revision: 9 })
})
afterEach(() => {
  cleanup()
  vi.resetAllMocks()
})
function mount(canEdit = true, value = school) {
  return render(
    <QueryClientProvider
      client={new QueryClient({ defaultOptions: { queries: { retry: false } } })}
    >
      <AuditModes school={value} canEdit={canEdit} />
    </QueryClientProvider>,
  )
}
async function chooseAutomatic() {
  const combo = await screen.findByRole('combobox', { name: '帖子审核模式' })
  fireEvent.mouseDown(combo)
  fireEvent.click((await screen.findAllByText('自动审核（自动通过）')).at(-1)!)
}
it('saves independent modes with the school revision and reports success', async () => {
  mount()
  await chooseAutomatic()
  fireEvent.click(screen.getByRole('button', { name: '保存审核模式' }))
  await waitFor(() =>
    expect(workspace.schoolProfiles.update).toHaveBeenCalledWith('school-1', {
      name: '学校甲',
      revision: 8,
      config: { post_audit_mode: 'auto', comment_audit_mode: 'manual' },
    }),
  )
  await screen.findByText('审核模式已保存')
})
it('keeps conflicted choices and requires explicit reload before another save', async () => {
  vi.mocked(workspace.schoolProfiles.update).mockRejectedValue(
    Object.assign(new Error('版本冲突'), { status: 409 }),
  )
  mount()
  await chooseAutomatic()
  fireEvent.click(screen.getByRole('button', { name: '保存审核模式' }))
  await screen.findByText('学校配置已变更，选择已保留，请重新加载后核对。')
  expect((screen.getByRole('button', { name: '保存审核模式' }) as HTMLButtonElement).disabled).toBe(
    true,
  )
  expect(screen.getByRole('combobox', { name: '帖子审核模式' }).closest('.ant-select')?.textContent).toContain('自动审核（自动通过）')
  vi.mocked(workspace.schoolProfiles.detail).mockResolvedValue({ ...school, revision: 10 })
  fireEvent.click(screen.getByRole('button', { name: '重新加载审核模式' }))
  await waitFor(() =>
    expect(
      (screen.getByRole('button', { name: '保存审核模式' }) as HTMLButtonElement).disabled,
    ).toBe(false),
  )
  expect(workspace.schoolProfiles.update).toHaveBeenCalledTimes(1)
})
it('operators can read modes but cannot save them', async () => {
  mount(false)
  const combo = await screen.findByRole('combobox', { name: '帖子审核模式' })
  expect((combo as HTMLInputElement).disabled).toBe(true)
  expect(screen.queryByRole('button', { name: '保存审核模式' })).toBeNull()
})
it('hidden schools cannot change modes even for administrators', async () => {
  vi.mocked(workspace.schoolProfiles.detail).mockResolvedValue({ ...school, status: 'hidden' })
  mount()
  const save = await screen.findByRole('button', { name: '保存审核模式' })
  expect((save as HTMLButtonElement).disabled).toBe(true)
})
