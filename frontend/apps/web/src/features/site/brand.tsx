import { Link } from 'react-router-dom'
import type { PublicSiteSettings } from './api'
import { useSiteSettings } from './provider'
import { text } from './title'

export function SiteBrand({
  value,
  to,
  className,
  suffix,
}: {
  value?: PublicSiteSettings
  to?: string
  className?: string
  suffix?: string
}) {
  const { settings } = useSiteSettings()
  const name = text((value ?? settings).site_name)
  if (!name) return null
  const label = suffix ? `${name} ${suffix}` : name
  const content = <strong>{label}</strong>
  return to ? (
    <Link className={className} to={to}>
      {content}
    </Link>
  ) : (
    <span className={className}>{content}</span>
  )
}
