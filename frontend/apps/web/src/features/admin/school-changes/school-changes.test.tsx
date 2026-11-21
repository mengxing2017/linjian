import { afterEach, expect, it, vi } from 'vitest'
import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react'
import { SchoolChanges } from './school-changes'
import { DocumentPreview } from './document-preview'
import { schoolChanges, type SchoolChangeRequest } from './api'

vi.mock('./api', () => ({ schoolChanges: { list: vi.fn(), detail: vi.fn(), document: vi.fn(), approve: vi.fn(), reject: vi.fn() } }))
const request: SchoolChangeRequest = {
  id: 'request', account_id: 'student', from_school_id: 'from', from_school_name: '原学校', to_school_id: 'to', to_school_name: '目标学校', status: 'pending', revision: 3,
  submitted_at: '2026-11-21T00:00:00Z', review_due_at: '2026-11-22T00:00:00Z', closed_at: null, reviewed_at: null, reviewed_by: null, consent_version: 'test-v1',
  personal_data_purged: false, declared_student_name: '测试申请人', declared_student_number: null, reason: '升学', result_reason: null, result_reason_code: null,
  documents: [{ id: 'document', mime: 'image/png', available: true }], blocked_reasons: [],
  student_relationship: 'new_student',
}
afterEach(() => { cleanup(); vi.resetAllMocks() })
async function mount(overrides: Partial<SchoolChangeRequest> = {}, reviewerId = 'reviewer', accountId?: string) {
  vi.mocked(schoolChanges.list).mockResolvedValue({ items: [request], page: { number: 1, size: 30, has_more: false } })
  vi.mocked(schoolChanges.detail).mockResolvedValue({ ...request, ...overrides })
  render(<SchoolChanges reviewerId={reviewerId} accountId={accountId} />)
  fireEvent.click(await screen.findByRole('button', { name: '查看申请' }))
  await screen.findByText('测试申请人')
}
it('keeps personal details out of the list and disables self-review', async () => {
  await mount({}, 'student')
  expect(screen.getByText('不能审核本人申请')).toBeTruthy()
  expect((screen.getByRole('button', { name: '批准并换校' }) as HTMLButtonElement).disabled).toBe(true)
  expect((screen.getByRole('button', { name: /驳\s*回/ }) as HTMLButtonElement).disabled).toBe(true)
  expect(schoolChanges.document).not.toHaveBeenCalled()
})
it('blocks approval on role conflicts even with a written conclusion', async () => {
  await mount({ blocked_reasons: ['role_conflict'] })
  fireEvent.change(screen.getByLabelText('审核结论或驳回原因'), { target: { value: '已核验目标学校' } })
  expect((screen.getByRole('button', { name: '批准并换校' }) as HTMLButtonElement).disabled).toBe(true)
  expect((screen.getByRole('button', { name: /驳\s*回/ }) as HTMLButtonElement).disabled).toBe(false)
})
it('drops stale review details after a version conflict rather than replaying the action', async () => {
  await mount()
  vi.mocked(schoolChanges.approve).mockRejectedValue(Object.assign(new Error('版本冲突，请刷新'), { status: 409 }))
  fireEvent.change(screen.getByLabelText('审核结论或驳回原因'), { target: { value: '确认有效' } })
  fireEvent.click(screen.getByRole('button', { name: '批准并换校' }))
  fireEvent.click(await screen.findByRole('button', { name: 'OK' }))
  await screen.findByText('版本冲突，请刷新')
  expect(schoolChanges.approve).toHaveBeenCalledWith(request, '确认有效')
  expect(screen.queryByLabelText('审核结论或驳回原因')).toBeNull()
  expect(schoolChanges.approve).toHaveBeenCalledTimes(1)
})
it('revokes private preview URLs on unmount and aborts the binary request', async () => {
  const create = vi.fn().mockReturnValue('blob:private-school-document'), revoke = vi.fn()
  Object.defineProperty(URL, 'createObjectURL', { configurable: true, value: create })
  Object.defineProperty(URL, 'revokeObjectURL', { configurable: true, value: revoke })
  vi.mocked(schoolChanges.document).mockResolvedValue(new ArrayBuffer(4))
  const view = render(<DocumentPreview request="request" document="document" />)
  await waitFor(() => expect(create).toHaveBeenCalledOnce())
  const signal = vi.mocked(schoolChanges.document).mock.calls[0][2]!
  view.unmount()
  expect(revoke).toHaveBeenCalledWith('blob:private-school-document')
  expect(signal.aborted).toBe(true)
})
it('does not submit a confirmation after its request detail has been closed', async () => {
  await mount()
  fireEvent.change(screen.getByLabelText('审核结论或驳回原因'), { target: { value: '确认有效' } })
  fireEvent.click(screen.getByRole('button', { name: '批准并换校' }))
  const confirm = await screen.findByRole('button', { name: 'OK' })
  fireEvent.click(screen.getByRole('button', { name: '关闭详情' }))
  fireEvent.click(confirm)
  await waitFor(() => expect(screen.queryByRole('button', { name: 'OK' })).toBeNull())
  expect(schoolChanges.approve).not.toHaveBeenCalled()
})
it('scopes the related-request queue to the account selected in user management', async () => {
  await mount({}, 'reviewer', 'student')
  expect(schoolChanges.list).toHaveBeenCalledWith(expect.objectContaining({ account_id: 'student' }), expect.any(AbortSignal))
  expect(screen.getByText(/当前仅查看账号 student/)).toBeTruthy()
})
it('shows automatic rejection as a system decision without offering review actions', async () => {
  await mount({ status: 'rejected', result_reason_code: 'review_timeout', result_reason: '超期未审核，自动驳回' })
  expect(screen.getByText(/系统因超期未审核自动驳回/)).toBeTruthy()
  expect(screen.getByText('新生')).toBeTruthy()
  expect(screen.queryByRole('button', { name: '批准并换校' })).toBeNull()
})
