import type { ReactNode } from 'react'
import { Link } from 'react-router-dom'
import { SiteFooter, text, useSiteSettings } from '../site'

export function CommunityShell({ children }: { children: ReactNode }) {
  const { settings } = useSiteSettings()
  const name = text(settings.site_name)
  return (
    <main className="community-shell">
      <header>
        <Link to="/community">{name ? `${name} 社区` : '校园社区'}</Link> ·{' '}
        <Link to="/admin">运营后台</Link>
      </header>
      <div className="community-content">{children}</div>
      <SiteFooter />
    </main>
  )
}
