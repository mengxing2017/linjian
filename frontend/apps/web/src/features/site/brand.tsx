import { useEffect, useState } from 'react'
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
  const current = value ?? settings
  const name = text(current.site_name)
  const logo = text(current.logo_url)
  const [failed, setFailed] = useState(false)
  useEffect(() => {
    setFailed(false)
  }, [logo])
  const showLogo = !!logo && !failed
  if (!name && !showLogo) return null
  const label = suffix ? `${name} ${suffix}` : name
  const content = (
    <>
      {showLogo && (
        <img
          className="site-brand-logo"
          src={logo}
          alt={name ? `${name} 标志` : '站点标志'}
          onError={() => setFailed(true)}
        />
      )}
      {name && <strong>{label}</strong>}
    </>
  )
  return to ? (
    <Link className={className} to={to}>
      {content}
    </Link>
  ) : (
    <span className={className}>{content}</span>
  )
}
