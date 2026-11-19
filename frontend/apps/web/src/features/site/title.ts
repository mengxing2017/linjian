import type { PublicSiteSettings } from './api'

export function text(value: string | null | undefined): string {
  return typeof value === 'string' ? value.trim() : ''
}

export function entryTitle(settings: PublicSiteSettings): string {
  return [text(settings.site_name), text(settings.title_subtitle)]
    .filter(Boolean)
    .join(' · ')
}

export function pageTitle(
  page: string | null | undefined,
  settings: PublicSiteSettings,
): string {
  return [text(page), text(settings.site_name)].filter(Boolean).join(' · ')
}
