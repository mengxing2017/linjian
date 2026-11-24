import { isRootId, type RootId } from '../../app/root-routes'
const KEY = 'linjian:miniapp:return-target:v1'
const UUID = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/
export type ReturnTarget = {
  kind: 'post-detail'
  school: string
  id: string
  createdAt: number
} | { kind: 'publish'; origin: RootId; account?: string; createdAt: number }
export function validPostTarget(school: string, id: string) {
  return UUID.test(school) && UUID.test(id)
}
export function clearReturnTarget() {
  uni.removeStorageSync(KEY)
}
export function saveReturnTarget(school: string, id: string) {
  if (!validPostTarget(school, id)) throw new Error('帖子链接无效')
  uni.setStorageSync(KEY, { kind: 'post-detail', school, id, createdAt: Date.now() })
}
export function savePublishTarget(origin: RootId, account?: string) {
  if (!isRootId(origin)) throw new Error('发布来源无效')
  uni.setStorageSync(KEY, { kind: 'publish', origin, account, createdAt: Date.now() })
}
export function readReturnTarget(): ReturnTarget | undefined {
  const value: unknown = uni.getStorageSync(KEY)
  if (!value) return
  if (typeof value !== 'object' || value === null) {
    clearReturnTarget()
    return
  }
  const v = value as Record<string, unknown>
  if (
    typeof v.createdAt !== 'number' ||
    !Number.isFinite(v.createdAt) ||
    Date.now() - v.createdAt > 30 * 60 * 1000 ||
    v.createdAt > Date.now() + 60000
  ) {
    clearReturnTarget()
    return
  }
  if (v.kind === 'publish' && isRootId(v.origin) && (v.account === undefined || typeof v.account === 'string')) {
    return { kind: 'publish', origin: v.origin, account: v.account, createdAt: v.createdAt }
  }
  if (v.kind === 'post-detail' && typeof v.school === 'string' && typeof v.id === 'string' && validPostTarget(v.school, v.id)) {
    return { kind: 'post-detail', school: v.school, id: v.id, createdAt: v.createdAt }
  }
  clearReturnTarget()
}
export function targetPath(value: ReturnTarget) {
  if (value.kind === 'publish') return '/pages/index/publish'
  if (!validPostTarget(value.school, value.id)) throw new Error('帖子链接无效')
  return `/pages/index/detail?school=${value.school}&id=${value.id}`
}
