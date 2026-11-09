import { afterEach, expect, it, vi } from 'vitest'
import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { MiniappUsers } from './index'
import { miniappUsers, type MiniappUser } from './api'
vi.mock('./api', () => ({
  miniappUsers: { list: vi.fn(), detail: vi.fn(), status: vi.fn(), avatar: vi.fn() },
}))
const user: MiniappUser = {
  id: 'user-1',
  nickname: '校园注册用户',
  avatar_url: null,
  avatar_asset_id: null,
  mobile: '13800138000',
  school_id: 'school-1',
  school_name: '示例学校',
  post_count: 3,
  created_at: '2026-11-09T00:00:00Z',
  status: 'normal',
  revision: 2,
}
afterEach(() => {
  cleanup()
  vi.resetAllMocks()
})
function mount(admin: boolean) {
  vi.mocked(miniappUsers.list).mockResolvedValue({ items: [user] })
  vi.mocked(miniappUsers.detail).mockResolvedValue(user)
  render(
    <QueryClientProvider
      client={new QueryClient({ defaultOptions: { queries: { retry: false } } })}
    >
      <MiniappUsers school="school-1" platformAdmin={false} schoolAdmin={admin} schools={[]} />
    </QueryClientProvider>,
  )
}
it('shows all seven attributes and keeps school operators read-only', async () => {
  mount(false)
  await screen.findByText('校园注册用户')
  for (const title of ['头像', '昵称', '手机号', '所属学校', '发帖数', '创建时间', '状态'])
    expect(screen.getByRole('columnheader', { name: new RegExp(title) })).toBeTruthy()
  expect(screen.getByText('13800138000')).toBeTruthy()
  fireEvent.click(screen.getByRole('button', { name: /详\s*情/ }))
  await screen.findByText('用户 ID', {}, { timeout: 5000 })
  expect(screen.queryByRole('button', { name: '封禁用户' })).toBeNull()
  expect(screen.queryByRole('button', { name: '纠正手机号' })).toBeNull()
  expect(miniappUsers.list).toHaveBeenCalledWith('school-1', {}, expect.any(AbortSignal))
})
it('requires a reason and submits the user revision when banning within the school', async () => {
  mount(true)
  await screen.findByText('校园注册用户')
  fireEvent.click(screen.getByRole('button', { name: /详\s*情/ }))
  await waitFor(() => expect(miniappUsers.detail).toHaveBeenCalled(), { timeout: 5000 })
  fireEvent.click(await screen.findByRole('button', { name: '封禁用户' }, { timeout: 5000 }))
  const confirm = (await screen.findByRole(
    'button',
    { name: 'OK' },
    { timeout: 5000 },
  )) as HTMLButtonElement
  expect(confirm.disabled).toBe(true)
  fireEvent.change(screen.getByLabelText('用户操作原因'), { target: { value: '违反学校规则' } })
  fireEvent.click(confirm)
  await waitFor(() =>
    expect(miniappUsers.status).toHaveBeenCalledWith('school-1', user, 'banned', '违反学校规则'),
  )
})
