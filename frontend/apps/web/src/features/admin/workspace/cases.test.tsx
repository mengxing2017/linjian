import { afterEach, expect, it, vi } from 'vitest'
import { cleanup, fireEvent, render, screen, waitFor, within } from '@testing-library/react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { Cases } from './cases'
import { workspace, ApiError, type CaseManagementView } from './api'
import { selectOption } from '../../../test/select-option'
vi.mock('./api', async importOriginal => ({ ...await importOriginal<typeof import('./api')>(), workspace: { cases: vi.fn(), resolveCase: vi.fn() } }))
afterEach(() => { cleanup(); vi.resetAllMocks() })
const record: CaseManagementView = { id: 'case', school_id: 'school', account_id: 'reporter', kind: 'feedback', post_id: null, content: '完整反馈原因', contact: '联系信息', images: [], state: 'open', resolution: '', revision: 1, created_at: '2026-11-10T01:00:00Z', title: '意见反馈', reporter_name: '提交人昵称', school_name: '甲学校', handled_at: null, handled_by: null }
function show() {
  const client = new QueryClient({ defaultOptions: { queries: { retry: false } } })
  render(<QueryClientProvider client={client}><Cases school="school" refresh={async () => {}} /></QueryClientProvider>)
}
it('uses exactly the seven business columns and applies state and search queries', async () => {
  vi.mocked(workspace.cases).mockResolvedValue({ items: [record] })
  show()
  await screen.findByText('完整反馈原因')
  expect(screen.getAllByRole('columnheader').map(h => h.textContent)).toEqual(['标题 / 名称', '举报原因', '举报人', '学校', '处理状态', '举报时间', '处理时间', '操作'])
  expect(screen.getByText('提交人昵称')).toBeTruthy()
  expect(screen.getByText('—')).toBeTruthy()
  await selectOption(screen.getByLabelText('处理状态'), '已驳回')
  await waitFor(() => expect(workspace.cases).toHaveBeenLastCalledWith('school', expect.stringContaining('status=dismissed'), expect.any(AbortSignal)))
  fireEvent.change(screen.getByLabelText('搜索内容'), { target: { value: '提交人昵称' } })
  fireEvent.keyDown(screen.getByLabelText('搜索内容'), { key: 'Enter', code: 'Enter' })
  await waitFor(() => {
    const query = new URLSearchParams(vi.mocked(workspace.cases).mock.lastCall![1])
    expect(query.get('search')).toBe('提交人昵称')
    expect(query.get('page[number]')).toBe('1')
  })
})
it('keeps a conflict draft and requires successful explicit reload before using the new revision', async () => {
  vi.mocked(workspace.cases).mockResolvedValue({ items: [record] })
  vi.mocked(workspace.resolveCase).mockRejectedValueOnce(new ApiError(409, 'conflict', '版本冲突'))
  show()
  const row = within((await screen.findByText('完整反馈原因')).closest('tr')!)
  fireEvent.click(row.getByRole('button', { name: /详\s*情/ }))
  const detail = within(await screen.findByRole('dialog'))
  fireEvent.change(detail.getByLabelText('处理说明'), { target: { value: '保留我的处理草稿' } })
  await selectOption(detail.getByLabelText('处理结果'), '已处理')
  fireEvent.click(detail.getByRole('button', { name: /^保\s*存$/ }))
  await detail.findByText('案件已发生变化，草稿已保留，请重新加载后再处理')
  expect(detail.getByRole('button', { name: /^保\s*存$/ })).toHaveProperty('disabled', true)
  vi.mocked(workspace.cases).mockRejectedValueOnce(new Error('重载失败'))
  fireEvent.click(detail.getByRole('button', { name: '重新加载案件' }))
  await waitFor(() => expect(detail.getByRole('button', { name: '重新加载案件' })).toHaveProperty('disabled', false))
  expect(detail.getByRole('button', { name: /^保\s*存$/ })).toHaveProperty('disabled', true)
  vi.mocked(workspace.cases).mockResolvedValue({ items: [{ ...record, revision: 2 }] })
  vi.mocked(workspace.resolveCase).mockResolvedValue({ ...record, state: 'resolved', revision: 3, resolution: '保留我的处理草稿', handled_at: '2026-11-10T02:00:00Z', handled_by: 'operator' })
  fireEvent.click(detail.getByRole('button', { name: '重新加载案件' }))
  await waitFor(() => expect(detail.getByRole('button', { name: /^保\s*存$/ })).toHaveProperty('disabled', false))
  expect(detail.getByLabelText('处理说明')).toHaveProperty('value', '保留我的处理草稿')
  fireEvent.click(detail.getByRole('button', { name: /^保\s*存$/ }))
  await waitFor(() => expect(workspace.resolveCase).toHaveBeenLastCalledWith('school', 'case', { revision: 2, state: 'resolved', resolution: '保留我的处理草稿' }))
})
