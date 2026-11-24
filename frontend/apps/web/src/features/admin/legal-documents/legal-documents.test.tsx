import { afterEach, expect, it, vi } from 'vitest'
import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react'
import { LegalDocuments } from './legal-documents'
import { legalDocuments, type LegalDocument } from './api'
vi.mock('./api', async importOriginal => ({ ...await importOriginal<typeof import('./api')>(), legalDocuments: { admin: vi.fn(), save: vi.fn(), clear: vi.fn(), history: vi.fn() } }))
const initial: LegalDocument = { id: 'user_agreement', kind: 'user_agreement', revision: 0, content: null, updated_at: '2026-11-24T00:00:00Z', updated_by_name: null }
afterEach(() => { cleanup(); vi.resetAllMocks() })
async function mount(document = initial) {
  vi.mocked(legalDocuments.admin).mockResolvedValue(document)
  render(<LegalDocuments />)
  const input = await screen.findByLabelText('用户协议正文') as HTMLTextAreaElement
  await waitFor(() => expect(input.disabled).toBe(false))
  return input
}
it('does not prefill an unconfigured document and rejects invisible content', async () => {
  const input = await mount()
  expect(input.value).toBe('')
  fireEvent.change(input, { target: { value: '\u200b　' } })
  expect((screen.getByRole('button', { name: '保存生效' }) as HTMLButtonElement).disabled).toBe(true)
})
it('saves the loaded revision only after explicit confirmation', async () => {
  vi.mocked(legalDocuments.save).mockResolvedValue({ ...initial, revision: 1, content: '正文\n第二段' })
  const input = await mount()
  fireEvent.change(input, { target: { value: '正文\n第二段' } })
  fireEvent.click(screen.getByRole('button', { name: '保存生效' }))
  expect(legalDocuments.save).not.toHaveBeenCalled()
  fireEvent.click(await screen.findByRole('button', { name: 'OK' }))
  await waitFor(() => expect(legalDocuments.save).toHaveBeenCalledWith('user_agreement', 0, '正文\n第二段'))
})
it('requires a separate clear command and keeps conflicting drafts', async () => {
  vi.mocked(legalDocuments.save).mockRejectedValue(new Error('版本冲突'))
  const input = await mount({ ...initial, revision: 3, content: '当前正文' })
  fireEvent.change(input, { target: { value: '本地草稿' } })
  fireEvent.click(screen.getByRole('button', { name: '保存生效' }))
  fireEvent.click(await screen.findByRole('button', { name: 'OK' }))
  await screen.findByText(/本地正文已保留/)
  expect(input.value).toBe('本地草稿')
  expect(legalDocuments.clear).not.toHaveBeenCalled()
})
