import roots from '../custom-tab-bar/routes.json'

export type RootId = 'home' | 'following' | 'messages' | 'profile'
export const rootRoutes = roots as { id: RootId; path: string; label: string }[]
export function isRootId(value: unknown): value is RootId {
  return rootRoutes.some(root => root.id === value)
}
export function rootForPath(path: string) {
  const normalized = `/${path.replace(/^\//, '').split('?')[0]}`
  return rootRoutes.find(root => root.path === normalized)
}
export function currentRoot() {
  const pages = getCurrentPages()
  return rootForPath(pages[pages.length - 1]?.route ?? '')?.id
}
