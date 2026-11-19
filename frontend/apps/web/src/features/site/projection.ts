import type {
  AdminSiteSettings,
  CopyrightEndMode,
  PublicSiteSettings,
  SiteSettingsInput,
} from './api'

export const EMPTY_SITE_SETTINGS: PublicSiteSettings = {
  revision: 0,
  site_name: null,
  logo_url: null,
  title_subtitle: null,
  login_intro: null,
  icp_number: null,
  police_number: null,
  police_url: null,
  copyright_owner: null,
  copyright_years: null,
  copyright_notice: null,
}

export function shanghaiYear(value = new Date()): number {
  return Number(
    new Intl.DateTimeFormat('en-US', {
      timeZone: 'Asia/Shanghai',
      year: 'numeric',
    }).format(value),
  )
}

export function copyrightYears(
  start: number | null,
  mode: CopyrightEndMode | null,
  end: number | null,
  currentYear = shanghaiYear(),
): string | null {
  if (!start || !mode) return null
  const last = mode === 'auto' ? currentYear : end
  if (!last) return null
  return last <= start ? String(start) : `${start}–${last}`
}

export function publicFromInput(
  input: SiteSettingsInput,
  logoUrl: string | null = null,
): PublicSiteSettings {
  return {
    revision: input.revision,
    site_name: input.site_name,
    logo_url: logoUrl,
    title_subtitle: input.title_subtitle,
    login_intro: input.login_intro,
    icp_number: input.icp_number,
    police_number: input.police_number,
    police_url: input.police_url,
    copyright_owner: input.copyright_owner,
    copyright_years: copyrightYears(
      input.copyright_start_year,
      input.copyright_end_mode,
      input.copyright_end_year,
    ),
    copyright_notice: input.copyright_notice,
  }
}

export function publicFromAdmin(settings: AdminSiteSettings): PublicSiteSettings {
  return {
    revision: settings.revision,
    site_name: settings.site_name,
    logo_url: settings.logo_url,
    title_subtitle: settings.title_subtitle,
    login_intro: settings.login_intro,
    icp_number: settings.icp_number,
    police_number: settings.police_number,
    police_url: settings.police_url,
    copyright_owner: settings.copyright_owner,
    copyright_years: settings.copyright_years,
    copyright_notice: settings.copyright_notice,
  }
}
