import type { ReactNode } from 'react'
import { Link } from 'react-router-dom'
import {
  SiteBrand,
  SiteFooter,
  text,
  useSiteSettings,
} from '../site'

export function CommunityShell({ children }: { children: ReactNode }) {
  const { settings } = useSiteSettings()
  const branded = !!(text(settings.site_name) || text(settings.logo_url))
  return (
    <main className="community-shell">
      <header>
        {branded ? (
          <SiteBrand to="/community" suffix="社区" />
        ) : (
          <Link to="/community">校园社区</Link>
        )}{' '}
        · <Link to="/admin">运营后台</Link>
      </header>
      <div className="community-content">{children}</div>
      <SiteFooter />
    </main>
  )
}
