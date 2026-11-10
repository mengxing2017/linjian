import { afterEach, beforeEach, expect, it, vi } from 'vitest'
import { cleanup, fireEvent, render, screen, waitFor, within } from '@testing-library/react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { OperationResources } from './resources'
import { OperationFeedback } from './feedback'
import { operationsApi as api, type OperationResource } from './api'
vi.mock('./api', () => ({ operationsApi: { list: vi.fn(), save: vi.fn(), remove: vi.fn(), feedback: vi.fn(), feedbackDetail: vi.fn(), deleteFeedback: vi.fn() }, workspace: { list: vi.fn() } }))
vi.mock('./media-picker', () => ({ MediaPicker: () => <div>本范围图片</div> }))
class Resize { observe() {} unobserve() {} disconnect() {} }
vi.stubGlobal('ResizeObserver', Resize)
Object.defineProperty(window, 'matchMedia', { writable: true, value: () => ({ matches: false, addListener: vi.fn(), removeListener: vi.fn(), addEventListener: vi.fn(), removeEventListener: vi.fn() }) })
const record: OperationResource = { id: 'ad', school_id: 'school', school_name: '本校', kind: 'banner', revision: 2, title: '校内广告', content: '', image_ref: { kind: 'asset', id: 'image' }, action: { kind: 'none' }, active: true, sort_order: 0, starts_at: null, ends_at: null, popup_strategy: 'daily', popup_interval_days: 1, image_url: null, views: 9, clicks: 3 }
beforeEach(() => { vi.clearAllMocks(); vi.mocked(api.list).mockResolvedValue({ items: [record] }) })
afterEach(cleanup)
function show(node: React.ReactNode) { render(<QueryClientProvider client={new QueryClient({ defaultOptions: { queries: { retry: false } } })}>{node}</QueryClientProvider>) }
it('shows all banner business columns and keeps operators read only', async () => {
  show(<OperationResources school="school" platformAdmin={false} schoolAdmin={false} module="home-banners" />)
  await screen.findByText('校内广告')
  expect(screen.getAllByRole('columnheader').map(v => v.textContent)).toEqual(['广告标题', '所属学校', '图片', '点击后去哪里', '目标内容', '状态', '开始展示时间', '结束展示时间', '曝光量', '点击量', '操作'])
  expect(screen.queryByRole('button', { name: '添加首页广告位' })).toBeNull()
  expect(screen.queryByRole('button', { name: /删\s*除/ })).toBeNull()
  fireEvent.change(screen.getByLabelText('名称查询'), { target: { value: '迎新' } })
  await waitFor(() => expect(api.list).toHaveBeenLastCalledWith('school', 'home-banners', expect.objectContaining({ search: '迎新', page: 1 }), expect.any(AbortSignal)))
})
it('preserves an editor and its revision after a conflicting save', async () => {
  vi.mocked(api.save).mockRejectedValue(new Error('版本冲突，请刷新'))
  show(<OperationResources school="school" platformAdmin={false} schoolAdmin module="home-banners" />)
  fireEvent.click(within((await screen.findByText('校内广告')).closest('tr')!).getByRole('button', { name: /编\s*辑/ }))
  const dialog = within(await screen.findByRole('dialog'))
  fireEvent.change(dialog.getByLabelText('广告标题'), { target: { value: '新的标题' } })
  fireEvent.click(dialog.getByRole('button', { name: /^预\s*览$/ }))
  fireEvent.click(dialog.getByRole('button', { name: '确认保存' }))
  await dialog.findByText('版本冲突，请刷新')
  expect(dialog.getByLabelText('广告标题')).toHaveProperty('value', '新的标题')
  expect(api.save).toHaveBeenCalledWith('school', 'home-banners', expect.objectContaining({ title: '新的标题', revision: 2 }), 'ad')
})
it('keeps feedback status read only and allows operators to inspect historic fields', async () => {
  const f = { id: 'f', school_id: 'school', account_id: 'u', school_name: '本校', reporter_name: '用户', feedback_type: null, content: '历史问题', contact: '', images: [], state: 'dismissed', submitted_ip: null, revision: 1, created_at: '2026-11-10T00:00:00Z', updated_at: '2026-11-10T00:00:00Z' } as const
  vi.mocked(api.feedback).mockResolvedValue({ items: [{ ...f, images: [] }] })
  show(<OperationFeedback school="school" canDelete={false} />)
  expect(await screen.findByText('历史反馈')).toBeTruthy()
  expect(screen.getByText('未记录')).toBeTruthy()
  expect(screen.getByText('已驳回')).toBeTruthy()
  expect(screen.queryByRole('button', { name: /删\s*除/ })).toBeNull()
})
