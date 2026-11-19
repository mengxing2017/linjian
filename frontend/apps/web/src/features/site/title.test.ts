import { describe, expect, it } from 'vitest'
import { EMPTY_SITE_SETTINGS, copyrightYears } from './projection'
import type { PublicSiteSettings } from './api'
import { entryTitle, pageTitle } from './title'

const full: PublicSiteSettings = {
  revision: 3,
  site_name: '示例校园',
  title_subtitle: '校园内容运营',
  login_intro: null,
  icp_number: null,
  police_number: null,
  police_url: null,
  copyright_owner: null,
  copyright_years: null,
  copyright_notice: null,
}

describe('site titles', () => {
  it('joins only non-empty entry values', () => {
    expect(entryTitle(full)).toBe('示例校园 · 校园内容运营')
    expect(entryTitle({ ...full, title_subtitle: null })).toBe('示例校园')
    expect(entryTitle({ ...full, site_name: '  ' })).toBe('校园内容运营')
    expect(entryTitle(EMPTY_SITE_SETTINGS)).toBe('')
  })

  it('appends the site name only when configured', () => {
    expect(pageTitle('内容管理', full)).toBe('内容管理 · 示例校园')
    expect(pageTitle('内容管理', { ...full, site_name: null })).toBe('内容管理')
    expect(pageTitle(null, full)).toBe('示例校园')
  })

  it('formats same-year and cross-year copyright ranges', () => {
    expect(copyrightYears(2026, 'auto', null, 2026)).toBe('2026')
    expect(copyrightYears(2024, 'auto', null, 2026)).toBe('2024–2026')
    expect(copyrightYears(2022, 'fixed', 2024, 2026)).toBe('2022–2024')
    expect(copyrightYears(null, 'auto', null, 2026)).toBeNull()
    expect(copyrightYears(2024, null, null, 2026)).toBeNull()
  })
})
