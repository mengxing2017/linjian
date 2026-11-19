import { PageErrorBoundary } from './page-error-boundary'
import { lazy, Suspense } from 'react'
import { Link, Navigate, Route, Routes } from 'react-router-dom'
import { SiteBrand, SiteFooter, useSiteTitle } from '../features/site'

const CommunityEntry = lazy(() => import('../features/community'))
const AdminEntry = lazy(() => import('../entries/admin'))

function NotFound() {
  useSiteTitle('页面不存在')
  return (
    <main className="not-found">
      <SiteBrand className="not-found-brand" to="/admin" />
      <h1>页面不存在</h1>
      <Link to="/admin">返回运营入口</Link>
      <SiteFooter />
    </main>
  )
}

export function AppRoutes() {
  return <PageErrorBoundary><Suspense fallback={<main className="loading" role="status">正在加载…</main>}>
    <Routes>
      <Route path="/" element={<Navigate to="/admin" replace />} />
      <Route path="/admin" element={<AdminEntry />} />
      <Route path="/community" element={<CommunityEntry />} />
      <Route path="/community/:school/posts/:id" element={<CommunityEntry />} />
      <Route path="*" element={<NotFound />} />
    </Routes>
  </Suspense></PageErrorBoundary>
}
