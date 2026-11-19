import { afterEach, beforeEach, expect, it, vi } from 'vitest'
import { cleanup, render, screen, waitFor } from '@testing-library/react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { siteSettings, type PublicSiteSettings } from './api'
import { SiteBrand } from './brand'
import { SiteFooter } from './footer'
import { SiteSettingsProvider } from './provider'
import { EMPTY_SITE_SETTINGS } from './projection'

vi.mock('./api', () => ({
  siteSettings: { public: vi.fn() },
}))

const full: PublicSiteSettings = {
  revision: 3,
  site_name: '示例校园',
  title_subtitle: '校园内容运营',
  login_intro: '连接学校、内容与社区',
  icp_number: '京ICP备00000000号-1',
  police_number: '京公网安备 11000000000000号',
  police_url: 'https://beian.mps.gov.cn/#/query/webSearch?code=1',
  copyright_owner: '示例运营主体',
  copyright_years: '2024–2026',
  copyright_notice: '保留所有权利',
}

beforeEach(() => {
  localStorage.clear()
})
afterEach(() => {
  cleanup()
  vi.resetAllMocks()
})

it('hides every empty footer group and the footer itself', () => {
  const empty = render(<SiteFooter value={EMPTY_SITE_SETTINGS} />)
  expect(empty.container.querySelector('footer')).toBeNull()
  empty.rerender(
    <SiteFooter value={{ ...EMPTY_SITE_SETTINGS, icp_number: '京ICP备1号' }} />,
  )
  expect(screen.getByText('京ICP备1号')).toBeTruthy()
  expect(screen.queryByText(/Copyright/)).toBeNull()
  expect(screen.queryByText(/京公网安备/)).toBeNull()
})

it('renders copyright, ICP query and police link with safe attributes', () => {
  render(<SiteFooter value={full} />)
  expect(
    screen.getByText('Copyright © 2024–2026 示例运营主体'),
  ).toBeTruthy()
  expect(screen.getByText('保留所有权利')).toBeTruthy()
  const icp = screen.getByRole('link', { name: '工信部备案查询' })
  expect(icp.getAttribute('href')).toBe('https://beian.miit.gov.cn/')
  const police = screen.getByRole('link', { name: /京公网安备/ })
  expect(police.getAttribute('href')).toBe(full.police_url)
  expect(police.getAttribute('target')).toBe('_blank')
  expect(police.getAttribute('rel')).toBe('noopener noreferrer')
})

it('renders no brand container when the site name is empty', () => {
  const view = render(<SiteBrand value={EMPTY_SITE_SETTINGS} />)
  expect(view.container.textContent).toBe('')
  view.rerender(<SiteBrand value={{ ...full, site_name: '  ' }} />)
  expect(view.container.textContent).toBe('')
})

it('loads public settings once and reuses the last successful value', async () => {
  vi.mocked(siteSettings.public).mockResolvedValue(full)
  const cache = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  })
  render(
    <QueryClientProvider client={cache}>
      <SiteSettingsProvider>
        <h1>登录工作空间</h1>
        <SiteBrand className="auth-brand" />
        <SiteFooter />
      </SiteSettingsProvider>
    </QueryClientProvider>,
  )
  await waitFor(() => expect(screen.getByText('示例校园')).toBeTruthy())
  expect(screen.getByText('京ICP备00000000号-1')).toBeTruthy()
  expect(localStorage.getItem('linjian.site-settings.v1')).toContain('示例校园')
  expect(siteSettings.public).toHaveBeenCalledTimes(1)
})
