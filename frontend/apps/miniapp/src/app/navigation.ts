import { session } from '../features/identity/session'
import { savePublishTarget, clearReturnTarget } from '../features/identity/return-target'
import { scopeKey } from '../features/school-context/context'
import { currentRoot, isRootId, rootForPath, rootRoutes, type RootId } from './root-routes'

let navigating: Promise<unknown> | undefined
let publishSource: { root: RootId; scope: string; account?: string } | undefined

function once(action: () => Promise<unknown>) {
  if (navigating) return navigating
  const operation = Promise.resolve().then(action)
  navigating = operation
  void operation.finally(() => { if (navigating === operation) navigating = undefined }).catch(() => {})
  return operation
}
export function switchRoot(root: RootId) {
  if (!isRootId(root)) return Promise.reject(new Error('导航目标无效'))
  if (currentRoot() === root) return Promise.resolve()
  return once(() => uni.switchTab({ url: rootRoutes.find(item => item.id === root)!.path }))
}
export function rememberPublishSource(root: RootId) {
  publishSource = { root, scope: scopeKey(), account: session.user?.id }
}
export function publishOrigin(): RootId {
  return publishSource?.scope === scopeKey() && publishSource.account === session.user?.id ? publishSource.root : 'home'
}
export function openPublish() {
  return once(() => {
    const root = currentRoot() ?? 'home'
    savePublishTarget(root, session.user?.id)
    rememberPublishSource(root)
    return uni.navigateTo({ url: '/pages/index/publish' })
  })
}
export function leavePublish() {
  clearReturnTarget()
  const root = publishOrigin()
  const pages = getCurrentPages()
  const previous = pages[pages.length - 2]
  if (previous && rootForPath(previous.route ?? '')?.id === root) return uni.navigateBack({ delta: 1 })
  return switchRoot(root)
}
export async function finishPublish(school: string, post: string) {
  const scope = scopeKey()
  const root = publishOrigin()
  const pages = getCurrentPages()
  const url = `/pages/index/detail?school=${encodeURIComponent(school)}&id=${encodeURIComponent(post)}`
  if (rootForPath(pages[pages.length - 2]?.route ?? '')?.id === root) {
    await uni.redirectTo({ url })
  } else {
    await switchRoot(root)
    if (scope === scopeKey()) await openPage(url)
  }
}
export function openPage(url: string) {
  const root = rootForPath(url)
  if (root) return switchRoot(root.id)
  if (url === '/pages/index/publish') return openPublish()
  return once(() => uni.navigateTo({ url }))
}
