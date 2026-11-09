import type { ReactNode } from 'react'
import { Link } from 'react-router-dom'

export function CommunityShell({ children }: { children: ReactNode }) {
  return (
    <main className="community-shell">
      <header>
        <Link to="/community">Linjian 社区</Link> · <Link to="/admin">运营后台</Link>
      </header>
      {children}
    </main>
  )
}
