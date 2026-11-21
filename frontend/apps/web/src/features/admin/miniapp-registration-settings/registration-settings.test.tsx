import { afterEach, expect, it, vi } from 'vitest'
import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react'
import { MiniappRegistrationSettings } from './registration-settings'
import { registrationSettings, type RegistrationSettings } from './api'

vi.mock('./api', () => ({ registrationSettings: { read: vi.fn(), update: vi.fn() } }))
const initial: RegistrationSettings = { nickname_prefix: '用户', revision: 0, updated_at: '2026-11-21T00:00:00Z', updated_by_name: null }
afterEach(() => { cleanup(); vi.resetAllMocks() })

async function mount() {
  vi.mocked(registrationSettings.read).mockResolvedValue(initial)
  render(<MiniappRegistrationSettings />)
  const input = await screen.findByLabelText('新用户昵称前缀') as HTMLInputElement
  await waitFor(() => expect(input.disabled).toBe(false))
  return input
}

it('loads the prefix, previews the final nickname and sends the loaded revision', async () => {
  vi.mocked(registrationSettings.update).mockResolvedValue({ ...initial, nickname_prefix: '校园用户', revision: 1, updated_by_name: '管理员' })
  const input = await mount()
  expect(input.value).toBe('用户')
  fireEvent.change(input, { target: { value: ' 校园用户 ' } })
  expect(screen.getByText(/示例：校园用户583194726/)).toBeTruthy()
  fireEvent.click(screen.getByRole('button', { name: '保存前缀' }))
  await waitFor(() => expect(registrationSettings.update).toHaveBeenCalledWith({ nickname_prefix: '校园用户', revision: 0 }))
  await waitFor(() => expect(input.value).toBe('校园用户'))
  expect(screen.getByText(/修订号：1/)).toBeTruthy()
})

it('validates Unicode scalar counts rather than UTF-16 length', async () => {
  const input = await mount()
  fireEvent.change(input, { target: { value: '😀'.repeat(91) } })
  expect((screen.getByRole('button', { name: '保存前缀' }) as HTMLButtonElement).disabled).toBe(false)
  fireEvent.change(input, { target: { value: '😀'.repeat(92) } })
  expect((screen.getByRole('button', { name: '保存前缀' }) as HTMLButtonElement).disabled).toBe(true)
  fireEvent.change(input, { target: { value: '   ' } })
  expect((screen.getByRole('button', { name: '保存前缀' }) as HTMLButtonElement).disabled).toBe(true)
})

it('retains conflicting input and reloads only after discarding is confirmed', async () => {
  vi.mocked(registrationSettings.update).mockRejectedValue(Object.assign(new Error('冲突'), { status: 409 }))
  const input = await mount()
  fireEvent.change(input, { target: { value: '草稿' } })
  fireEvent.click(screen.getByRole('button', { name: '保存前缀' }))
  await screen.findByText('配置已变化，当前输入已保留，请重新加载后编辑')
  expect(input.value).toBe('草稿')
  expect((screen.getByRole('button', { name: '保存前缀' }) as HTMLButtonElement).disabled).toBe(true)
  vi.mocked(registrationSettings.read).mockResolvedValue({ ...initial, nickname_prefix: '另一端', revision: 2 })
  fireEvent.click(screen.getByRole('button', { name: '重新加载' }))
  const discard = await screen.findByRole('button', { name: '丢弃并重新加载' })
  expect(registrationSettings.read).toHaveBeenCalledTimes(1)
  fireEvent.click(discard)
  await waitFor(() => expect(input.value).toBe('另一端'))
})

it('disables editing and duplicate submissions while saving', async () => {
  let finish!: (value: RegistrationSettings) => void
  vi.mocked(registrationSettings.update).mockReturnValue(new Promise((resolve) => { finish = resolve }))
  const input = await mount()
  fireEvent.change(input, { target: { value: '校园' } })
  fireEvent.click(screen.getByRole('button', { name: /保存前缀/ }))
  expect(input.disabled).toBe(true)
  fireEvent.click(screen.getByRole('button', { name: /保存前缀/ }))
  expect(registrationSettings.update).toHaveBeenCalledTimes(1)
  finish({ ...initial, nickname_prefix: '校园', revision: 1 })
  await waitFor(() => expect(input.disabled).toBe(false))
})
