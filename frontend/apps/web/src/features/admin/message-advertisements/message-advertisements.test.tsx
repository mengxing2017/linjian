import { afterEach, beforeEach, expect, it, vi } from 'vitest'
import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react'
import { readFileSync } from 'node:fs'
import { URL as NodeURL } from 'node:url'
import { messageAdvertisements, type MessageAdvertisement, type AdvertisementRevision } from './api'
import { SchoolMessageAdvertisement } from './school-message-advertisement'
import { AdvertisementReviewDetail } from './platform-message-advertisement-reviews'

vi.mock('./api', async original => ({
  ...await original<typeof import('./api')>(),
  messageAdvertisements: { current: vi.fn(), history: vi.fn(), create: vi.fn(), saveDraft: vi.fn(), submit: vi.fn(), withdraw: vi.fn(), remove: vi.fn(), detail: vi.fn(), decide: vi.fn(), removePlatform: vi.fn(), list: vi.fn() },
}))

function fixture(): MessageAdvertisement {
  const document = JSON.parse(readFileSync(new NodeURL('../../../../../../../api-contract/fixtures/message-advertisement-current.json', import.meta.url), 'utf8'))
  return { id: document.data.id, ...document.data.attributes }
}
function pending(): AdvertisementRevision {
  const config = fixture()
  const candidate: AdvertisementRevision = { ...config.current_revision!, id: 'candidate', content: '【广告】候选新文案', status: 'pending', number: '2', version: '2', config_version: '5', decision: null }
  return { ...candidate, configuration: { ...config, version: '5', candidate } }
}
beforeEach(() => {
  vi.mocked(messageAdvertisements.current).mockResolvedValue(fixture())
  vi.mocked(messageAdvertisements.history).mockResolvedValue({ items: [], page: { number: 1, size: 30, has_more: false } })
  vi.mocked(messageAdvertisements.detail).mockResolvedValue(pending())
})
afterEach(() => { cleanup(); vi.resetAllMocks() })

it('keeps school read-only access separate from management', async () => {
  render(<SchoolMessageAdvertisement school="school-a" canManage={false} />)
  await screen.findByText('通过时间：2026-11-24T00:00:00Z')
  expect((screen.getByLabelText('广告文案') as HTMLTextAreaElement).disabled).toBe(true)
  expect(screen.queryByRole('button', { name: '保存草稿' })).toBeNull()
  expect(screen.queryByRole('button', { name: '删除广告' })).toBeNull()
  expect(messageAdvertisements.create).not.toHaveBeenCalled()
})

it('shows the approved version while a different candidate waits for platform review', async () => {
  vi.mocked(messageAdvertisements.current).mockResolvedValue(pending().configuration!)
  render(<SchoolMessageAdvertisement school="school-a" canManage />)
  await screen.findByText('候选文案 · 待平台审核')
  expect(screen.getByText('【广告】本校打印服务')).toBeTruthy()
  expect((screen.getByLabelText('广告文案') as HTMLTextAreaElement).value).toBe('【广告】候选新文案')
  expect((screen.getByLabelText('广告文案') as HTMLTextAreaElement).disabled).toBe(true)
  expect(screen.queryByRole('button', { name: '提交平台审核' })).toBeNull()
})

it('preserves edited text on conflict and requires explicit reload before manual retry', async () => {
  vi.mocked(messageAdvertisements.saveDraft).mockRejectedValue(Object.assign(new Error('版本冲突'), { status: 409 }))
  render(<SchoolMessageAdvertisement school="school-a" canManage />)
  await screen.findByText('通过时间：2026-11-24T00:00:00Z')
  fireEvent.change(screen.getByLabelText('广告文案'), { target: { value: '【广告】保留输入' } })
  fireEvent.click(screen.getByRole('button', { name: '保存草稿' }))
  await screen.findByText('数据已变化，当前文案已保留。请重新加载并核对，再手动保存或提交。')
  expect((screen.getByLabelText('广告文案') as HTMLTextAreaElement).value).toBe('【广告】保留输入')
  expect(messageAdvertisements.saveDraft).toHaveBeenCalledTimes(1)
  const updated = { ...fixture(), version: '6' }
  vi.mocked(messageAdvertisements.current).mockResolvedValue(updated)
  fireEvent.click(screen.getByRole('button', { name: '重新加载并核对' }))
  await waitFor(() => expect((screen.getByRole('button', { name: '保存草稿' }) as HTMLButtonElement).disabled).toBe(false))
  expect((screen.getByLabelText('广告文案') as HTMLTextAreaElement).value).toBe('【广告】保留输入')
  fireEvent.click(screen.getByRole('button', { name: '保存草稿' }))
  await waitFor(() => expect(messageAdvertisements.saveDraft).toHaveBeenLastCalledWith('school-a', updated, '【广告】保留输入'))
})

it('isolates late reads after changing schools', async () => {
  let resolve!: (value: MessageAdvertisement) => void
  vi.mocked(messageAdvertisements.current).mockReturnValueOnce(new Promise(done => { resolve = done })).mockResolvedValueOnce(null)
  const view = render(<SchoolMessageAdvertisement school="school-a" canManage />)
  const signal = vi.mocked(messageAdvertisements.current).mock.calls[0][1]!
  view.rerender(<SchoolMessageAdvertisement school="school-b" canManage />)
  await screen.findByText('暂无已通过广告，原消息正常推送')
  resolve(fixture())
  await waitFor(() => expect(signal.aborted).toBe(true))
  expect((screen.getByLabelText('广告文案') as HTMLTextAreaElement).value).toBe('')
})

it('does not let a completed old-school mutation clear the next school form', async () => {
  let resolve!: (value: MessageAdvertisement) => void
  vi.mocked(messageAdvertisements.saveDraft).mockReturnValue(new Promise(done => { resolve = done }))
  const view = render(<SchoolMessageAdvertisement school="school-a" canManage />)
  await screen.findByText('通过时间：2026-11-24T00:00:00Z')
  fireEvent.change(screen.getByLabelText('广告文案'), { target: { value: '【广告】旧校修改' } })
  fireEvent.click(screen.getByRole('button', { name: '保存草稿' }))
  vi.mocked(messageAdvertisements.current).mockResolvedValueOnce(null)
  view.rerender(<SchoolMessageAdvertisement school="school-b" canManage />)
  await screen.findByText('暂无已通过广告，原消息正常推送')
  fireEvent.change(screen.getByLabelText('广告文案'), { target: { value: '【广告】新校输入' } })
  resolve(fixture())
  await waitFor(() => expect((screen.getByLabelText('广告文案') as HTMLTextAreaElement).value).toBe('【广告】新校输入'))
})

it('keeps review conflicts visible without silently approving a new version', async () => {
  vi.mocked(messageAdvertisements.decide).mockRejectedValue(Object.assign(new Error('审核冲突'), { status: 409 }))
  render(<AdvertisementReviewDetail revision="candidate" onClose={vi.fn()} onDone={vi.fn()} />)
  await screen.findByText('【广告】候选新文案')
  fireEvent.change(screen.getByLabelText('广告审核或下架说明'), { target: { value: '已阅读全文' } })
  fireEvent.click(screen.getByRole('button', { name: '通过审核' }))
  await screen.findByText('修订已变化，说明已保留；重新加载全文并核对后才能继续审核。')
  expect((screen.getByRole('button', { name: '通过审核' }) as HTMLButtonElement).disabled).toBe(true)
  expect((screen.getByLabelText('广告审核或下架说明') as HTMLTextAreaElement).value).toBe('已阅读全文')
  expect(messageAdvertisements.decide).toHaveBeenCalledTimes(1)
})

it('requires reasons for rejection and platform removal', async () => {
  render(<AdvertisementReviewDetail revision="candidate" onClose={vi.fn()} onDone={vi.fn()} />)
  await screen.findByText('【广告】候选新文案')
  expect((screen.getByRole('button', { name: '驳回广告' }) as HTMLButtonElement).disabled).toBe(true)
  expect((screen.getByRole('button', { name: '强制下架' }) as HTMLButtonElement).disabled).toBe(true)
  fireEvent.change(screen.getByLabelText('广告审核或下架说明'), { target: { value: '请修改' } })
  fireEvent.click(screen.getByRole('button', { name: '驳回广告' }))
  await waitFor(() => expect(messageAdvertisements.decide).toHaveBeenCalledWith(pending(), 'reject', '请修改'))
})
