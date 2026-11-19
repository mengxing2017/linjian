import { afterEach, expect, it, vi } from 'vitest'
import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { SiteSettings } from './site-settings-page'
import { siteSettings, type AdminSiteSettings } from '../../site'

vi.mock('../../site', async (importOriginal) => {
  const actual = await importOriginal<typeof import('../../site')>()
  return {
    ...actual,
    siteSettings: { read: vi.fn(), update: vi.fn() },
  }
})

const empty: AdminSiteSettings = {
  revision: 0,
  site_name: null,
  title_subtitle: null,
  login_intro: null,
  icp_number: null,
  police_number: null,
  police_url: null,
  copyright_owner: null,
  copyright_start_year: null,
  copyright_end_mode: null,
  copyright_end_year: null,
  copyright_years: null,
  copyright_notice: null,
  updated_at: '2026-11-19T00:00:00Z',
  updated_by_name: null,
  history: [],
}

afterEach(() => {
  cleanup()
  vi.resetAllMocks()
})

function mount() {
  const cache = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  })
  render(
    <QueryClientProvider client={cache}>
      <SiteSettings />
    </QueryClientProvider>,
  )
}

it('saves every field with the loaded revision', async () => {
  vi.mocked(siteSettings.read).mockResolvedValue(empty)
  vi.mocked(siteSettings.update).mockResolvedValue({
    ...empty,
    revision: 1,
    site_name: '示例校园',
  })
  mount()
  const input = (await screen.findByLabelText('网站名称')) as HTMLInputElement
  await waitFor(() => expect(input.disabled).toBe(false))
  fireEvent.change(input, { target: { value: '示例校园' } })
  fireEvent.click(screen.getByRole('button', { name: '保存并生效' }))
  await waitFor(() => expect(siteSettings.update).toHaveBeenCalledTimes(1))
  expect(vi.mocked(siteSettings.update).mock.calls[0][0]).toMatchObject({
    revision: 0,
    site_name: '示例校园',
    title_subtitle: null,
    icp_number: null,
    police_number: null,
    police_url: null,
    copyright_owner: null,
  })
  await screen.findByText('已保存并生效，当前版本 1')
})

it('keeps local input when another administrator already saved', async () => {
  vi.mocked(siteSettings.read).mockResolvedValue(empty)
  vi.mocked(siteSettings.update).mockRejectedValue({
    status: 409,
    message: '内容已变化，请刷新后重试',
  })
  mount()
  const input = (await screen.findByLabelText('网站名称')) as HTMLInputElement
  await waitFor(() => expect(input.disabled).toBe(false))
  fireEvent.change(input, { target: { value: '校园新名称' } })
  fireEvent.click(screen.getByRole('button', { name: '保存并生效' }))
  await screen.findByText(/配置已被其他管理员更新/)
  expect(input.value).toBe('校园新名称')
  expect(screen.getByRole('button', { name: '重新加载' })).toBeTruthy()
})
