import type { HomeMenuQuery } from './types'
export function homeMenuQuery(q: HomeMenuQuery): string {
  const p = new URLSearchParams()
  if (q.search) p.set('search', q.search)
  if (q.menu_status) p.set('menu_status', q.menu_status)
  if (q.source_type) p.set('source_type', q.source_type)
  if (q.enable_status) p.set('enable_status', q.enable_status)
  if (q.page) p.set('page[number]', String(q.page))
  return p.toString()
}
